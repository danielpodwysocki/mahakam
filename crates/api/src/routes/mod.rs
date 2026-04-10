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
    /// OCI image reference for the Android emulator viewer container.
    pub android_viewer_image: Arc<String>,
    /// Git repository URL containing `chart/workspace/` (pulled by ArgoCD).
    pub repo_url: Arc<String>,
    /// Git revision ArgoCD tracks for the workspace chart.
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
            "/api/v1/workspaces",
            get(handlers::workspaces::list_workspaces).post(handlers::workspaces::create_workspace),
        )
        .route(
            "/api/v1/workspaces/{name}",
            get(handlers::workspaces::get_workspace).delete(handlers::workspaces::delete_workspace),
        )
        .route(
            "/api/v1/workspaces/{name}/metrics",
            get(handlers::workspaces::get_workspace_metrics),
        )
        .route("/api/v1/projects", get(handlers::projects::list_projects))
        .route(
            "/api/v1/projects/{name}/workspaces",
            get(handlers::projects::list_project_workspaces),
        )
        .with_state(state)
        .layer(CorsLayer::permissive())
}
