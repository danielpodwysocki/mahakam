use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use clap::Parser;
use sqlx::sqlite::SqlitePoolOptions;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod k8s;
mod routes;

/// HTTP API service for mahakam.
#[derive(Parser)]
struct Config {
    #[arg(long, env = "API_PORT", default_value = "3000")]
    port: u16,

    #[arg(
        long,
        env = "DATABASE_URL",
        default_value = "sqlite:///data/mahakam.db"
    )]
    database_url: String,

    #[arg(
        long,
        env = "ENVIRONMENTS_BASE_PATH",
        default_value = "/app/environments/base"
    )]
    environments_base_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::parse();

    // Ensure parent directory for SQLite database exists.
    let db_path = config
        .database_url
        .trim_start_matches("sqlite://")
        .trim_start_matches("sqlite:");
    if let Some(parent) = PathBuf::from(db_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let kube_client = kube::Client::try_default().await?;

    let state = routes::AppState {
        pool,
        kube_client: Arc::new(kube_client),
        base_path: Arc::new(config.environments_base_path),
    };

    let app = routes::router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
