//! Explicit helpers for isolated PostgreSQL integration tests.

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/// Stable identifiers returned by [`seed_control_plane`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct SeedIds {
    /// Seed project identifier.
    pub project_id: Uuid,
    /// Seed environment identifier.
    pub environment_id: Uuid,
}

/// Removes application rows while preserving `SQLx`'s migration ledger.
pub async fn reset_test_data(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "TRUNCATE TABLE
        reporting.report_artifacts, reporting.report_metadata, reporting.recommendation_dispositions,
            reporting.recommendation_evidence, reporting.recommendation_diagnostics,
            reporting.recommendations, reporting.diagnostic_evidence, reporting.diagnostics,
            reporting.execution_metadata, reporting.status_observations, reporting.simulations,
            reporting.submission_attempts, reporting.trace_aliases, reporting.traces,
            reporting.business_actions, work.jobs, telemetry.raw_events, telemetry.event_dedup,
            telemetry.ingest_batches,
            control.api_tokens, control.routes, control.environments, control.projects,
            control.audit_log, control.workspace_invitations, control.workspace_members,
            control.workspaces, identity.sessions, identity.password_credentials, identity.users
         RESTART IDENTITY CASCADE",
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Inserts a deterministic project and environment fixture in one transaction.
pub async fn seed_control_plane(pool: &PgPool) -> Result<SeedIds, sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    let ids = SeedIds {
        project_id: Uuid::from_u128(0x018f_2d8e_7b3a_7c01_8a01_0000_0000_0001),
        environment_id: Uuid::from_u128(0x018f_2d8e_7b3a_7c01_8a01_0000_0000_0002),
    };
    sqlx::query("INSERT INTO control.projects (project_id, name) VALUES ($1, $2)")
        .bind(ids.project_id)
        .bind("test-project")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO control.environments (environment_id, project_id, name, cluster) VALUES ($1, $2, $3, $4)")
        .bind(ids.environment_id)
        .bind(ids.project_id)
        .bind("test")
        .bind("devnet")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ids)
}
