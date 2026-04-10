use std::{path::PathBuf, sync::Arc};

use axum::{routing::get, Router};
use tower_http::cors::CorsLayer;

pub mod handlers;

/// Shared application state injected into all handlers.
#[derive(Clone)]
pub struct AppState {
    pub kube_client: Arc<kube::Client>,
    pub base_path: Arc<PathBuf>,
    /// OCI image reference for the terminal viewer (ttyd) container.
    pub viewer_image: Arc<String>,
    /// OCI image reference for the browser viewer (noVNC) container.
    pub browser_viewer_image: Arc<String>,
    /// Git repository URL containing `chart/environment/` (pulled by ArgoCD).
    pub repo_url: Arc<String>,
    /// Git revision ArgoCD tracks for the environment chart.
    pub repo_revision: Arc<String>,
    /// Namespace where ArgoCD is installed.
    pub argocd_namespace: Arc<String>,
    /// vcluster Helm chart version pinned in the inner Application.
    pub vcluster_chart_version: Arc<String>,
}

/// Constructs the top-level application router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/health", get(handlers::health::health_check))
        .route(
            "/api/v1/environments",
            get(handlers::environments::list_environments)
                .post(handlers::environments::create_environment),
        )
        .route(
            "/api/v1/environments/{name}",
            axum::routing::delete(handlers::environments::delete_environment),
        )
        .with_state(state)
        .layer(CorsLayer::permissive())
}
