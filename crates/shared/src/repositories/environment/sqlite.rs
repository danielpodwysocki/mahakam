use async_trait::async_trait;
use sqlx::SqlitePool;

use super::{Environment, EnvironmentRepository};

/// SQLite-backed implementation of [`EnvironmentRepository`].
pub struct SqliteEnvironmentRepository {
    pool: SqlitePool,
}

impl SqliteEnvironmentRepository {
    /// Creates a new repository using the given connection pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// Intermediate row type for sqlx deserialization.
#[derive(sqlx::FromRow)]
struct EnvironmentRow {
    id: String,
    name: String,
    repos: String,
    namespace: String,
    status: String,
    created_at: String,
}

fn row_to_env(row: EnvironmentRow) -> anyhow::Result<Environment> {
    let repos: Vec<String> = serde_json::from_str(&row.repos)?;
    Ok(Environment {
        id: row.id,
        name: row.name,
        repos,
        namespace: row.namespace,
        status: row.status,
        created_at: row.created_at,
    })
}

#[async_trait]
impl EnvironmentRepository for SqliteEnvironmentRepository {
    async fn create(&self, env: &Environment) -> anyhow::Result<()> {
        let repos_json = serde_json::to_string(&env.repos)?;
        sqlx::query(
            "INSERT INTO environments (id, name, repos, namespace, status, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&env.id)
        .bind(&env.name)
        .bind(&repos_json)
        .bind(&env.namespace)
        .bind(&env.status)
        .bind(&env.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list(&self) -> anyhow::Result<Vec<Environment>> {
        let rows = sqlx::query_as::<_, EnvironmentRow>(
            "SELECT id, name, repos, namespace, status, created_at FROM environments \
             ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_env).collect()
    }

    async fn find_by_name(&self, name: &str) -> anyhow::Result<Option<Environment>> {
        let row = sqlx::query_as::<_, EnvironmentRow>(
            "SELECT id, name, repos, namespace, status, created_at FROM environments \
             WHERE name = ?",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_env).transpose()
    }

    async fn delete(&self, name: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM environments WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_status(&self, name: &str, status: &str) -> anyhow::Result<()> {
        sqlx::query("UPDATE environments SET status = ? WHERE name = ?")
            .bind(status)
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
