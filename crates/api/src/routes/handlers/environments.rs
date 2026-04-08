use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use tracing::error;

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

/// POST /api/v1/environments — creates an environment and provisions K8s resources.
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

    // K8s provisioning — best-effort for MVP (log errors, return 500 if they occur).
    if let Err(e) = crate::k8s::namespace::create_env_namespace(&state.kube_client, &env.name).await
    {
        error!("Failed to create namespace for {}: {}", env.name, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    if let Err(e) = crate::k8s::vcluster::install_vcluster(&env.name).await {
        error!("Failed to install vcluster for {}: {}", env.name, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    if let Err(e) =
        crate::k8s::kustomize::apply_env_kustomization(&env.name, &env.repos, &state.base_path)
            .await
    {
        error!("Failed to apply kustomization for {}: {}", env.name, e);
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    (StatusCode::CREATED, Json(env)).into_response()
}

/// DELETE /api/v1/environments/:name — tears down K8s resources and removes the record.
pub async fn delete_environment(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
