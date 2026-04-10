use async_trait::async_trait;

pub mod mock;
pub mod models;
pub mod sqlite;

/// A viewer endpoint (e.g. terminal, browser) attached to a workspace.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Viewer {
    pub name: String,
    pub display_name: String,
    pub path: String,
}

/// A managed workspace with an associated vcluster.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub repos: Vec<String>,
    pub namespace: String,
    pub status: String,
    pub created_at: String,
    /// Viewer endpoints discovered from HTTPRoute labels at list time.
    pub viewers: Vec<Viewer>,
    /// Logical project grouping for this workspace.
    pub project: String,
}

/// Repository trait for workspace persistence.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkspaceRepository: Send + Sync {
    async fn create(&self, ws: &Workspace) -> anyhow::Result<()>;
    async fn list(&self) -> anyhow::Result<Vec<Workspace>>;
    async fn find_by_name(&self, name: &str) -> anyhow::Result<Option<Workspace>>;
    async fn delete(&self, name: &str) -> anyhow::Result<()>;
    async fn update_status(&self, name: &str, status: &str) -> anyhow::Result<()>;
}
