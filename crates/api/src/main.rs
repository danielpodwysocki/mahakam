use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

mod routes;

/// HTTP API service for rust-template.
#[derive(Parser)]
struct Config {
    #[arg(long, env = "API_PORT", default_value = "3000")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::parse();

    let app = routes::router();

    let addr = format!("0.0.0.0:{}", config.port);
    info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
