//! Service-facing repository traits and their `SQLx` implementations.

use async_trait::async_trait;
use sqlx::PgPool;

/// Storage operation failure boundary exposed to services.
#[derive(Debug)]
pub enum RepositoryError {
    /// SQLx/database failure.
    Database(sqlx::Error),
}
impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(error) => write!(f, "database operation failed: {error}"),
        }
    }
}
impl std::error::Error for RepositoryError {}
impl From<sqlx::Error> for RepositoryError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

/// Minimal service boundary used by health checks and later repositories.
#[async_trait]
pub trait PersistenceRepository: Send + Sync {
    /// Verifies that the database connection can execute a trivial query.
    async fn health_check(&self) -> Result<(), RepositoryError>;
}

/// PostgreSQL-backed repository root shared by concrete repositories.
#[derive(Clone, Debug)]
pub struct PgRepositories {
    pool: PgPool,
}

impl PgRepositories {
    /// Wraps an `SQLx` pool without performing I/O.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    /// Returns the underlying pool for narrowly scoped implementations.
    #[must_use]
    pub const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl PersistenceRepository for PgRepositories {
    async fn health_check(&self) -> Result<(), RepositoryError> {
        sqlx::query("SELECT 1").execute(&self.pool).await?;
        Ok(())
    }
}
