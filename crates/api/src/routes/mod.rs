use std::{path::PathBuf, sync::Arc};

use axum::{routing::get, Router};
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;

pub mod handlers;

/// Shared application state injected into all handlers.
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub kube_client: Arc<kube::Client>,
    pub base_path: Arc<PathBuf>,
    /// OCI image reference for the viewer (ttyd) container spawned per environment.
    pub viewer_image: Arc<String>,
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
