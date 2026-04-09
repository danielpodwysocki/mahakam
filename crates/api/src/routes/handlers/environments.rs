use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use tracing::{error, info};
use uuid::Uuid;

use shared::{repositories::environment::Environment, services::environment::validate_name};

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub repos: Vec<String>,
}

/// GET /api/v1/environments — returns all environments.
///
/// Reads directly from ArgoCD Application annotations; survives API pod restarts.
pub async fn list_environments(State(state): State<AppState>) -> impl IntoResponse {
    match crate::k8s::argocd::list_env_applications(&state.kube_client, &state.argocd_namespace)
        .await
    {
        Ok(envs) => (StatusCode::OK, Json(envs)).into_response(),
        Err(e) => {
            error!("Failed to list environments: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

/// POST /api/v1/environments — creates an ArgoCD Application and returns immediately.
///
/// The ArgoCD Application carries all environment metadata as annotations so the
/// record survives pod restarts. Provisioning (waiting for vcluster, applying
/// kustomization, spawning viewer) runs in the background and updates the status
/// annotation when it settles.
pub async fn create_environment(
    State(state): State<AppState>,
    Json(body): Json<CreateEnvironmentRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_name(&body.name) {
        return (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()).into_response();
    }

    let env = Environment {
        id: Uuid::new_v4().to_string(),
        namespace: format!("env-{}", body.name),
        name: body.name,
        repos: body.repos,
        status: "pending".to_string(),
        created_at: Utc::now().to_rfc3339(),
    };

    // Create the ArgoCD Application now (synchronous) so the environment is
    // immediately visible in list even before provisioning completes.
    if let Err(e) = crate::k8s::argocd::create_env_application(
        &state.kube_client,
        &crate::k8s::argocd::EnvApplicationSpec {
            env_name: &env.name,
            env_id: &env.id,
            env_repos: &env.repos,
            env_created_at: &env.created_at,
            repo_url: &state.repo_url,
            repo_revision: &state.repo_revision,
            argocd_namespace: &state.argocd_namespace,
            argocd_project: "default",
            vcluster_chart_version: &state.vcluster_chart_version,
        },
    )
    .await
    {
        error!(env = %env.name, error = %e, "failed to create ArgoCD Application");
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    let kube_client = state.kube_client.clone();
    let base_path = state.base_path.clone();
    let viewer_image = state.viewer_image.clone();
    let argocd_namespace = state.argocd_namespace.clone();
    let env_name = env.name.clone();
    let env_repos = env.repos.clone();

    tokio::spawn(async move {
        info!(env = %env_name, "background provisioning started");
        let result: anyhow::Result<()> = async {
            crate::k8s::argocd::wait_for_env_healthy(&kube_client, &env_name, &argocd_namespace)
                .await?;

            let kubeconfig =
                crate::k8s::vcluster::wait_for_vcluster_kubeconfig(&kube_client, &env_name).await?;

            crate::k8s::vcluster::wait_for_vcluster_api_ready(&env_name, &kubeconfig).await?;

            crate::k8s::kustomize::apply_env_kustomization(
                &env_name,
                &env_repos,
                &base_path,
                &kubeconfig,
            )
            .await?;

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

        if let Err(e) = crate::k8s::argocd::update_env_application_status(
            &kube_client,
            &env_name,
            &argocd_namespace,
            new_status,
        )
        .await
        {
            error!(env = %env_name, error = %e, "failed to update status annotation");
        }
    });

    (StatusCode::CREATED, Json(env)).into_response()
}

/// DELETE /api/v1/environments/:name
///
/// Deletes the viewer HTTPRoute then the outer ArgoCD Application (cascade:
/// the finalizer unwinds the inner vcluster Application and the namespace).
pub async fn delete_environment(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = crate::k8s::viewer::teardown_viewer(&name).await {
        error!("Failed to teardown viewer for {}: {}", name, e);
    }

    match crate::k8s::argocd::delete_env_application(
        &state.kube_client,
        &name,
        &state.argocd_namespace,
    )
    .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            error!("Failed to delete ArgoCD Application for {}: {}", name, e);
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}
