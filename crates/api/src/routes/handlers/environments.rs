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

/// POST /api/v1/environments — creates a DB record, provisions the namespace and vcluster,
/// then applies the base kustomization inside the vcluster.
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

    // Provisioning runs in the background; the environment is returned immediately
    // with status "pending" so the UI can show it right away.
    let pool = state.pool.clone();
    let kube_client = state.kube_client.clone();
    let base_path = state.base_path.clone();
    let viewer_image = state.viewer_image.clone();
    let env_name = env.name.clone();
    let env_repos = env.repos.clone();

    tokio::spawn(async move {
        info!(env = %env_name, "background provisioning started");
        let result: anyhow::Result<()> = async {
            crate::k8s::namespace::create_env_namespace(&kube_client, &env_name).await?;
            crate::k8s::vcluster::install_vcluster(&env_name).await?;
            let kubeconfig =
                crate::k8s::vcluster::wait_for_vcluster_kubeconfig(&kube_client, &env_name).await?;
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

        let repo = SqliteEnvironmentRepository::new(pool);
        let svc = EnvironmentService::new(repo);
        if let Err(e) = svc.update_status(&env_name, new_status).await {
            error!(env = %env_name, error = %e, "failed to persist status update");
        }
    });

    (StatusCode::CREATED, Json(env)).into_response()
}

/// DELETE /api/v1/environments/:name — uninstalls the vcluster, tears down the namespace,
/// and removes the DB record.
pub async fn delete_environment(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Remove the viewer HTTPRoute first. Best-effort: log but don't block deletion.
    if let Err(e) = crate::k8s::viewer::teardown_viewer(&name).await {
        error!("Failed to teardown viewer for {}: {}", name, e);
    }

    if let Err(e) = crate::k8s::vcluster::uninstall_vcluster(&name).await {
        error!("Failed to uninstall vcluster for {}: {}", name, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    if let Err(e) = crate::k8s::namespace::delete_env_namespace(&state.kube_client, &name).await {
        error!("Failed to delete namespace for {}: {}", name, e);
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
