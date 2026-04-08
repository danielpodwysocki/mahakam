use axum::{routing::get, Router};

pub mod handlers;

/// Constructs the top-level application router.
pub fn router() -> Router {
    Router::new().route("/api/v1/health", get(handlers::health::health_check))
}
