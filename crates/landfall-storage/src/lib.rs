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
pub mod trace_events;
pub use trace_events::{RawEventRow, load_events_for_trace};
pub mod projections;
pub use projections::{AttemptProjectionWrite, TraceProjectionWrite, replace_trace_projection};
pub mod diagnostics;
pub use diagnostics::{DiagnosticWrite, append_diagnostics};
pub mod recommendations;
pub use recommendations::{RecommendationWrite, append_recommendations};
pub mod watermarks;
pub use watermarks::advance_projection_watermark;
pub mod observation_jobs;
pub use observation_jobs::{
    ClaimedObservationJob, claim_observation_job, complete_observation_job,
    enqueue_observation_if_eligible, reclaim_expired_observation_jobs, retry_observation_job,
};
pub mod repositories;
pub use repositories::{PersistenceRepository, PgRepositories, RepositoryError};
pub mod partitions;
pub use partitions::{
    drop_raw_event_partition, drop_raw_event_partitions_before, ensure_raw_event_partition,
};
pub mod test_support;
pub use test_support::{SeedIds, reset_test_data, seed_control_plane};
