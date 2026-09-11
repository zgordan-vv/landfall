//! PostgreSQL repositories, migrations, and durable job-queue adapter.

pub mod config;
pub use config::{ConfigError, DatabaseConfig};
pub mod repositories;
pub use repositories::{PersistenceRepository, PgRepositories, RepositoryError};
