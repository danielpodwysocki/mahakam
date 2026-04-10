use async_trait::async_trait;
use sqlx::SqlitePool;

use super::{Workspace, WorkspaceRepository};

/// SQLite-backed implementation of [`WorkspaceRepository`].
pub struct SqliteWorkspaceRepository {
    pool: SqlitePool,
}

impl SqliteWorkspaceRepository {
    /// Creates a new repository using the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// Intermediate row type for sqlx deserialization.
#[derive(sqlx::FromRow)]
struct WorkspaceRow {
    id: String,
    name: String,
    repos: String,
    namespace: String,
    status: String,
    created_at: String,
}

fn row_to_ws(row: WorkspaceRow) -> anyhow::Result<Workspace> {
    let repos: Vec<String> = serde_json::from_str(&row.repos)?;
    Ok(Workspace {
        id: row.id,
        name: row.name,
        repos,
        namespace: row.namespace,
        status: row.status,
        created_at: row.created_at,
        viewers: vec![],
        project: "default".to_string(),
    })
}

#[async_trait]
impl WorkspaceRepository for SqliteWorkspaceRepository {
    async fn create(&self, ws: &Workspace) -> anyhow::Result<()> {
        let repos_json = serde_json::to_string(&ws.repos)?;
        sqlx::query(
            "INSERT INTO workspaces (id, name, repos, namespace, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&ws.id)
        .bind(&ws.name)
        .bind(&repos_json)
        .bind(&ws.namespace)
        .bind(&ws.status)
        .bind(&ws.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list(&self) -> anyhow::Result<Vec<Workspace>> {
        let rows = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT id, name, repos, namespace, status, created_at FROM workspaces \
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_ws).collect()
    }

    async fn find_by_name(&self, name: &str) -> anyhow::Result<Option<Workspace>> {
        let row = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT id, name, repos, namespace, status, created_at FROM workspaces \
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_ws).transpose()
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM workspaces WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_status(&self, name: &str, status: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE workspaces SET status = ? WHERE name = ?")
            .bind(status)
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
