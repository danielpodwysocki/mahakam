use anyhow::{anyhow, Result};
use chrono::Utc;
use uuid::Uuid;

use crate::repositories::environment::{Environment, EnvironmentRepository};

/// Business logic for environment lifecycle management.
pub struct EnvironmentService<R: EnvironmentRepository> {
    repo: R,
}

impl<R: EnvironmentRepository> EnvironmentService<R> {
    /// Creates a new service backed by the given repository.
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Validates `name`, persists a new environment, and returns it.
    ///
    /// Name rules: 1–63 characters, only lowercase ASCII letters, digits, and hyphens.
    pub async fn create(&self, name: String, repos: Vec<String>) -> Result<Environment> {
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
        let id = Uuid::new_v4().to_string();
        let namespace = format!("env-{name}");
        let env = Environment {
            id,
            name,
            repos,
            namespace,
            status: "pending".to_string(),
            created_at: Utc::now().to_rfc3339(),
        };
        self.repo.create(&env).await?;
        Ok(env)
    }

    /// Returns all environments.
    pub async fn list(&self) -> Result<Vec<Environment>> {
        self.repo.list().await
    }

    /// Returns a single environment by name, or `None` if not found.
    pub async fn find_by_name(&self, name: &str) -> Result<Option<Environment>> {
        self.repo.find_by_name(name).await
    }

    /// Deletes an environment by name.
    pub async fn delete(&self, name: &str) -> Result<()> {
        self.repo.delete(name).await
    }

    /// Updates the status of an environment.
    pub async fn update_status(&self, name: &str, status: &str) -> Result<()> {
        self.repo.update_status(name, status).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::environment::mock::MockEnvironmentRepository;

    // ── create ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn create_valid_name_succeeds() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_create().once().returning(|_| Ok(()));
        let svc = EnvironmentService::new(mock);
        let result = svc
            .create(
                "my-env".to_string(),
                vec!["https://github.com/foo/bar".to_string()],
            )
            .await;
        assert!(result.is_ok());
        let env = result.unwrap();
        assert_eq!(env.name, "my-env");
        assert_eq!(env.namespace, "env-my-env");
        assert_eq!(env.status, "pending");
    }

    #[tokio::test]
    async fn create_max_length_name_succeeds() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_create().once().returning(|_| Ok(()));
        let svc = EnvironmentService::new(mock);
        let name = "a".repeat(63);
        let result = svc.create(name.clone(), vec![]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, name);
    }

    #[tokio::test]
    async fn create_empty_name_is_rejected() {
        let mock = MockEnvironmentRepository::new();
        let svc = EnvironmentService::new(mock);
        let err = svc.create("".to_string(), vec![]).await.unwrap_err();
        assert!(err.to_string().contains("1 and 63"));
    }

    #[tokio::test]
    async fn create_name_too_long_is_rejected() {
        let mock = MockEnvironmentRepository::new();
        let svc = EnvironmentService::new(mock);
        let err = svc.create("a".repeat(64), vec![]).await.unwrap_err();
        assert!(err.to_string().contains("1 and 63"));
    }

    #[tokio::test]
    async fn create_name_with_uppercase_is_rejected() {
        let mock = MockEnvironmentRepository::new();
        let svc = EnvironmentService::new(mock);
        let err = svc.create("MyEnv".to_string(), vec![]).await.unwrap_err();
        assert!(err.to_string().contains("lowercase"));
    }

    #[tokio::test]
    async fn create_name_with_space_is_rejected() {
        let mock = MockEnvironmentRepository::new();
        let svc = EnvironmentService::new(mock);
        assert!(svc.create("my env".to_string(), vec![]).await.is_err());
    }

    #[tokio::test]
    async fn create_name_with_special_chars_is_rejected() {
        let mock = MockEnvironmentRepository::new();
        let svc = EnvironmentService::new(mock);
        assert!(svc.create("my_env!".to_string(), vec![]).await.is_err());
    }

    #[tokio::test]
    async fn create_propagates_repo_error() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_create()
            .once()
            .returning(|_| Err(anyhow!("db error")));
        let svc = EnvironmentService::new(mock);
        assert!(svc.create("my-env".to_string(), vec![]).await.is_err());
    }

    // ── list ─────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn list_delegates_to_repo() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_list().once().returning(|| Ok(vec![]));
        let svc = EnvironmentService::new(mock);
        assert!(svc.list().await.unwrap().is_empty());
    }

    // ── find_by_name ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn find_by_name_delegates_to_repo() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_find_by_name().once().returning(|_| Ok(None));
        let svc = EnvironmentService::new(mock);
        assert!(svc.find_by_name("test").await.unwrap().is_none());
    }

    // ── delete ───────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_delegates_to_repo() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_delete().once().returning(|_| Ok(()));
        let svc = EnvironmentService::new(mock);
        assert!(svc.delete("test").await.is_ok());
    }

    // ── update_status ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn update_status_delegates_to_repo() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_update_status().once().returning(|_, _| Ok(()));
        let svc = EnvironmentService::new(mock);
        assert!(svc.update_status("test", "ready").await.is_ok());
    }

    #[tokio::test]
    async fn update_status_propagates_repo_error() {
        let mut mock = MockEnvironmentRepository::new();
        mock.expect_update_status()
            .once()
            .returning(|_, _| Err(anyhow!("db error")));
        let svc = EnvironmentService::new(mock);
        assert!(svc.update_status("test", "ready").await.is_err());
    }
}
