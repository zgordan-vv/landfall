//! PostgreSQL repositories, migrations, and durable job-queue adapter.

pub mod config;
pub use config::{ConfigError, DatabaseConfig};
pub mod migrations;
pub use migrations::{
    MIGRATION_SET_VERSION, MIGRATOR, MigrationHealth, expected_migration_versions,
    migration_health, run_migrations,
};
pub mod ingestion;
pub use ingestion::{IngestEvent, IngestOutcome, ingest_atomically};
pub mod repositories;
pub use repositories::{PersistenceRepository, PgRepositories, RepositoryError};
pub mod partitions;
pub use partitions::{
    drop_raw_event_partition, drop_raw_event_partitions_before, ensure_raw_event_partition,
};
pub mod test_support;
pub use test_support::{SeedIds, reset_test_data, seed_control_plane};
