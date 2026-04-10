use anyhow::{anyhow, Result};
use chrono::Utc;
use uuid::Uuid;

use crate::repositories::workspace::{Workspace, WorkspaceRepository};

/// Validates a workspace name.
///
/// Rules: 1–63 characters, only lowercase ASCII letters, digits, and hyphens.
pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 63 {
        return Err(anyhow!("name must be between 1 and 63 characters"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(anyhow!(
            "name must contain only lowercase letters, digits, and hyphens"
        ));
    }
    Ok(())
}

/// Business logic for workspace lifecycle management.
pub struct WorkspaceService<R: WorkspaceRepository> {
    repo: R,
}

impl<R: WorkspaceRepository> WorkspaceService<R> {
    /// Creates a new service backed by the given repository.
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Validates `name`, persists a new workspace, and returns it.
    ///
    /// Name rules: 1–63 characters, only lowercase ASCII letters, digits, and hyphens.
    pub async fn create(&self, name: String, repos: Vec<String>) -> Result<Workspace> {
        validate_name(&name)?;
        let id = Uuid::new_v4().to_string();
        let namespace = format!("ws-{name}");
        let ws = Workspace {
            id,
            name,
            repos,
            namespace,
            status: "pending".to_string(),
            created_at: Utc::now().to_rfc3339(),
            viewers: vec![],
            project: "default".to_string(),
        };
        self.repo.create(&ws).await?;
        Ok(ws)
    }

    /// Returns all workspaces.
    pub async fn list(&self) -> Result<Vec<Workspace>> {
        self.repo.list().await
    }

    /// Returns a single workspace by name, or `None` if not found.
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Workspace>> {
        self.repo.find_by_name(name).await
    }

    /// Deletes a workspace by name.
    pub async fn delete(&self, name: &str) -> Result<()> {
        self.repo.delete(name).await
    }

    /// Updates the status of a workspace.
    pub async fn update_status(&self, name: &str, status: &str) -> Result<()> {
        self.repo.update_status(name, status).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::workspace::mock::MockWorkspaceRepository;

    // ── create ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_valid_name_succeeds() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_create().once().returning(|_| Ok(()));
        let svc = WorkspaceService::new(mock);
        let result = svc
            .create(
                "my-ws".to_string(),
                vec!["https://github.com/foo/bar".to_string()],
            )
            .await;
        assert!(result.is_ok());
        let ws = result.unwrap();
        assert_eq!(ws.name, "my-ws");
        assert_eq!(ws.namespace, "ws-my-ws");
        assert_eq!(ws.status, "pending");
    }

    #[tokio::test]
    async fn create_max_length_name_succeeds() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_create().once().returning(|_| Ok(()));
        let svc = WorkspaceService::new(mock);
        let name = "a".repeat(63);
        let result = svc.create(name.clone(), vec![]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, name);
    }

    #[tokio::test]
    async fn create_empty_name_is_rejected() {
        let mock = MockWorkspaceRepository::new();
        let svc = WorkspaceService::new(mock);
        let err = svc.create("".to_string(), vec![]).await.unwrap_err();
        assert!(err.to_string().contains("1 and 63"));
    }

    #[tokio::test]
    async fn create_name_too_long_is_rejected() {
        let mock = MockWorkspaceRepository::new();
        let svc = WorkspaceService::new(mock);
        let err = svc.create("a".repeat(64), vec![]).await.unwrap_err();
        assert!(err.to_string().contains("1 and 63"));
    }

    #[tokio::test]
    async fn create_name_with_uppercase_is_rejected() {
        let mock = MockWorkspaceRepository::new();
        let svc = WorkspaceService::new(mock);
        let err = svc.create("MyWs".to_string(), vec![]).await.unwrap_err();
        assert!(err.to_string().contains("lowercase"));
    }

    #[tokio::test]
    async fn create_name_with_space_is_rejected() {
        let mock = MockWorkspaceRepository::new();
        let svc = WorkspaceService::new(mock);
        assert!(svc.create("my ws".to_string(), vec![]).await.is_err());
    }

    #[tokio::test]
    async fn create_name_with_special_chars_is_rejected() {
        let mock = MockWorkspaceRepository::new();
        let svc = WorkspaceService::new(mock);
        assert!(svc.create("my_ws!".to_string(), vec![]).await.is_err());
    }

    #[tokio::test]
    async fn create_propagates_repo_error() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_create()
            .once()
            .returning(|_| Err(anyhow!("db error")));
        let svc = WorkspaceService::new(mock);
        assert!(svc.create("my-ws".to_string(), vec![]).await.is_err());
    }

    // ── list ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_delegates_to_repo() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_list().once().returning(|| Ok(vec![]));
        let svc = WorkspaceService::new(mock);
        assert!(svc.list().await.unwrap().is_empty());
    }

    // ── find_by_name ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_name_delegates_to_repo() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_find_by_name().once().returning(|_| Ok(None));
        let svc = WorkspaceService::new(mock);
        assert!(svc.find_by_name("test").await.unwrap().is_none());
    }

    // ── delete ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_delegates_to_repo() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_delete().once().returning(|_| Ok(()));
        let svc = WorkspaceService::new(mock);
        assert!(svc.delete("test").await.is_ok());
    }

    // ── update_status ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_status_delegates_to_repo() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_update_status().once().returning(|_, _| Ok(()));
        let svc = WorkspaceService::new(mock);
        assert!(svc.update_status("test", "ready").await.is_ok());
    }

    #[tokio::test]
    async fn update_status_propagates_repo_error() {
        let mut mock = MockWorkspaceRepository::new();
        mock.expect_update_status()
            .once()
            .returning(|_, _| Err(anyhow!("db error")));
        let svc = WorkspaceService::new(mock);
        assert!(svc.update_status("test", "ready").await.is_err());
    }
}
