//! PostgreSQL repositories, migrations, and durable job-queue adapter.

pub mod config;
pub use config::{ConfigError, DatabaseConfig};
pub mod migrations;
pub use migrations::{
    MIGRATION_SET_VERSION, MIGRATOR, MigrationHealth, expected_migration_versions,
    migration_health, run_migrations,
};
pub mod repositories;
pub use repositories::{PersistenceRepository, PgRepositories, RepositoryError};
pub mod test_support;
pub use test_support::{SeedIds, reset_test_data, seed_control_plane};
