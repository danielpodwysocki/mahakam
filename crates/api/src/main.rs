use std::{path::PathBuf, sync::Arc};

use anyhow::Result;
use clap::Parser;
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
        env = "ENVIRONMENTS_BASE_PATH",
        default_value = "/app/environments/base"
    )]
    environments_base_path: PathBuf,

    #[arg(long, env = "VIEWER_IMAGE", default_value = "mahakam-ttyd:latest")]
    viewer_image: String,

    #[arg(
        long,
        env = "REPO_URL",
        default_value = "https://github.com/danielpodwysocki/mahakam.git"
    )]
    repo_url: String,

    #[arg(long, env = "REPO_REVISION", default_value = "HEAD")]
    repo_revision: String,

    #[arg(long, env = "ARGOCD_NAMESPACE", default_value = "argocd")]
    argocd_namespace: String,

    #[arg(long, env = "VCLUSTER_CHART_VERSION", default_value = "0.33.1")]
    vcluster_chart_version: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::parse();
    let kube_client = kube::Client::try_default().await?;

    let state = routes::AppState {
        kube_client: Arc::new(kube_client),
        base_path: Arc::new(config.environments_base_path),
        viewer_image: Arc::new(config.viewer_image),
        repo_url: Arc::new(config.repo_url),
        repo_revision: Arc::new(config.repo_revision),
        argocd_namespace: Arc::new(config.argocd_namespace),
        vcluster_chart_version: Arc::new(config.vcluster_chart_version),
    };

    let app = routes::router(state);

    let addr = format!("0.0.0.0:{}", config.port);
    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
