use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::{error, info};

use shared::{
    repositories::environment::sqlite::SqliteEnvironmentRepository,
    services::environment::EnvironmentService,
};

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub repos: Vec<String>,
}

/// GET /api/v1/environments — returns all environments.
pub async fn list_environments(State(state): State<AppState>) -> impl IntoResponse {
    let repo = SqliteEnvironmentRepository::new(state.pool.clone());
    let svc = EnvironmentService::new(repo);
    match svc.list().await {
        Ok(envs) => (StatusCode::OK, Json(envs)).into_response(),
        Err(e) => {
            error!("Failed to list environments: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// POST /api/v1/environments — creates a DB record and returns immediately.
///
/// Provisioning runs in the background:
/// 1. An outer ArgoCD Application (`env-{name}`) is created, which manages:
///    - wave -1: Namespace `env-{name}` (labeled for mahakam)
///    - wave  0: Inner Application `vcluster-{name}` (installs the vcluster Helm chart)
/// 2. We wait for the outer Application to reach Healthy+Synced.
/// 3. The vcluster kubeconfig secret is retrieved and the base kustomization is applied.
/// 4. A ttyd viewer Deployment/Service/Route is spawned in the environment namespace.
pub async fn create_environment(
    State(state): State<AppState>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> impl IntoResponse {
    let repo = SqliteEnvironmentRepository::new(state.pool.clone());
    let svc = EnvironmentService::new(repo);

    let env = match svc.create(body.name, body.repos).await {
        Ok(env) => env,
        Err(e) => {
            error!("Failed to create environment: {}", e);
            return (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response();
        }
    };

    let pool = state.pool.clone();
    let kube_client = state.kube_client.clone();
    let base_path = state.base_path.clone();
    let viewer_image = state.viewer_image.clone();
    let repo_url = state.repo_url.clone();
    let repo_revision = state.repo_revision.clone();
    let argocd_namespace = state.argocd_namespace.clone();
    let vcluster_chart_version = state.vcluster_chart_version.clone();
    let env_name = env.name.clone();
    let env_repos = env.repos.clone();

    tokio::spawn(async move {
        info!(env = %env_name, "background provisioning started");
        let result: anyhow::Result<()> = async {
            // Create the outer ArgoCD Application; ArgoCD handles namespace + vcluster.
            crate::k8s::argocd::create_env_application(
                &kube_client,
                &crate::k8s::argocd::EnvApplicationSpec {
                    env_name: &env_name,
                    repo_url: &repo_url,
                    repo_revision: &repo_revision,
                    argocd_namespace: &argocd_namespace,
                    argocd_project: "default",
                    vcluster_chart_version: &vcluster_chart_version,
                },
            )
            .await?;

            // Block until namespace + vcluster are reconciled and healthy.
            crate::k8s::argocd::wait_for_env_healthy(&kube_client, &env_name, &argocd_namespace)
                .await?;

            // Retrieve the kubeconfig the vcluster chart wrote as a Secret.
            let kubeconfig =
                crate::k8s::vcluster::wait_for_vcluster_kubeconfig(&kube_client, &env_name).await?;

            // Wait for the vcluster API server to accept connections — the pod may be
            // Running (and thus Healthy in ArgoCD) before the API server is ready.
            crate::k8s::vcluster::wait_for_vcluster_api_ready(&env_name, &kubeconfig).await?;

            // Apply the base kustomization overlay inside the vcluster.
            crate::k8s::kustomize::apply_env_kustomization(
                &env_name,
                &env_repos,
                &base_path,
                &kubeconfig,
            )
            .await?;

            // Spawn the ttyd console viewer in the environment namespace.
            crate::k8s::viewer::spawn_viewer(
                &kube_client,
                &env_name,
                crate::k8s::viewer::ViewerSpec {
                    image: (*viewer_image).clone(),
                    path_prefix: format!("/projects/viewers/{env_name}"),
                    port: 7681,
                    env_vars: vec![("ENV_NAME".to_string(), env_name.clone())],
                },
            )
            .await?;

            Ok(())
        }
        .await;

        let new_status = match result {
            Ok(()) => {
                info!(env = %env_name, "provisioning complete");
                "ready"
            }
            Err(ref e) => {
                error!(env = %env_name, error = %e, "provisioning failed");
                "failed"
            }
        };

        let repo = SqliteEnvironmentRepository::new(pool);
        let svc = EnvironmentService::new(repo);
        if let Err(e) = svc.update_status(&env_name, new_status).await {
            error!(env = %env_name, error = %e, "failed to persist status update");
        }
    });

    (StatusCode::CREATED, Json(env)).into_response()
}

/// DELETE /api/v1/environments/:name
///
/// Deletes the viewer HTTPRoute, then the outer ArgoCD Application (cascade:
/// the finalizer unwinds the inner vcluster Application and the namespace),
/// then removes the DB record.
pub async fn delete_environment(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Remove the viewer HTTPRoute first (it lives in mahakam-system, not the env namespace,
    // so ArgoCD cascade won't reach it).
    if let Err(e) = crate::k8s::viewer::teardown_viewer(&name).await {
        error!("Failed to teardown viewer for {}: {}", name, e);
    }

    // Delete the outer Application; its finalizer cascades through the inner Application
    // (vcluster Helm release) and then the namespace — in reverse-wave order.
    if let Err(e) = crate::k8s::argocd::delete_env_application(
        &state.kube_client,
        &name,
        &state.argocd_namespace,
    )
    .await
    {
        error!("Failed to delete ArgoCD Application for {}: {}", name, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    let repo = SqliteEnvironmentRepository::new(state.pool.clone());
    let svc = EnvironmentService::new(repo);
    match svc.delete(&name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("Failed to delete environment record for {}: {}", name, e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}
