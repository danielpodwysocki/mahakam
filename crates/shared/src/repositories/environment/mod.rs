use async_trait::async_trait;

pub mod mock;
pub mod models;
pub mod sqlite;

/// A managed environment with an associated vcluster.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub repos: Vec<String>,
    pub namespace: String,
    pub status: String,
    pub created_at: String,
}

/// Repository trait for environment persistence.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait EnvironmentRepository: Send + Sync {
    async fn create(&self, env: &Environment) -> anyhow::Result<()>;
    async fn list(&self) -> anyhow::Result<Vec<Environment>>;
    async fn find_by_name(&self, name: &str) -> anyhow::Result<Option<Environment>>;
    async fn delete(&self, name: &str) -> anyhow::Result<()>;
    async fn update_status(&self, name: &str, status: &str) -> anyhow::Result<()>;
}
