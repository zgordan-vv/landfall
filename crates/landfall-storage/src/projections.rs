//! Transactional persistence of reduced trace read models.
#![allow(missing_docs)]

use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct TraceProjectionWrite {
    pub trace_id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Uuid,
    pub lifecycle_state: String,
    pub landing_state: String,
    pub execution_state: String,
    pub application_state: String,
    pub observation_state: String,
    pub updated_at: OffsetDateTime,
    pub attempts: Vec<AttemptProjectionWrite>,
}

#[derive(Debug, Clone)]
pub struct AttemptProjectionWrite {
    pub attempt_id: Uuid,
    pub route_id: Option<Uuid>,
    pub started_event_id: Option<Uuid>,
    pub completed_event_id: Option<Uuid>,
    pub created_at: OffsetDateTime,
}

/// Replaces one trace and its typed attempt children atomically.
pub async fn replace_trace_projection(
    pool: &PgPool,
    projection: &TraceProjectionWrite,
) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    sqlx::query("INSERT INTO reporting.traces (trace_id, project_id, environment_id, lifecycle_state, landing_state, execution_state, application_state, observation_state, updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) ON CONFLICT (trace_id) DO UPDATE SET project_id=EXCLUDED.project_id, environment_id=EXCLUDED.environment_id, lifecycle_state=EXCLUDED.lifecycle_state, landing_state=EXCLUDED.landing_state, execution_state=EXCLUDED.execution_state, application_state=EXCLUDED.application_state, observation_state=EXCLUDED.observation_state, updated_at=EXCLUDED.updated_at")
        .bind(projection.trace_id).bind(projection.project_id).bind(projection.environment_id).bind(&projection.lifecycle_state).bind(&projection.landing_state).bind(&projection.execution_state).bind(&projection.application_state).bind(&projection.observation_state).bind(projection.updated_at).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM reporting.submission_attempts WHERE trace_id = $1")
        .bind(projection.trace_id)
        .execute(&mut *tx)
        .await?;
    for attempt in &projection.attempts {
        sqlx::query("INSERT INTO reporting.submission_attempts (attempt_id, trace_id, route_id, started_event_id, completed_event_id, created_at) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(attempt.attempt_id).bind(projection.trace_id).bind(attempt.route_id).bind(attempt.started_event_id).bind(attempt.completed_event_id).bind(attempt.created_at).execute(&mut *tx).await?;
    }
    tx.commit().await
}
