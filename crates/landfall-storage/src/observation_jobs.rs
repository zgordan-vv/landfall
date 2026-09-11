//! Eligibility-gated observation job enqueueing.

use sqlx::PgPool;
use uuid::Uuid;

/// Enqueues one deduplicated observation job only for an eligible trace.
pub async fn enqueue_observation_if_eligible(
    pool: &PgPool,
    trace_id: Uuid,
    eligible: bool,
) -> Result<bool, sqlx::Error> {
    if !eligible {
        return Ok(false);
    }
    let result = sqlx::query("INSERT INTO work.jobs (job_id, job_type, dedupe_key, payload) VALUES ($1, 'observe_trace', $2, jsonb_build_object('trace_id', $3)) ON CONFLICT (job_type, dedupe_key) WHERE status IN ('ready', 'running') DO NOTHING")
        .bind(Uuid::now_v7()).bind(trace_id.to_string()).bind(trace_id).execute(pool).await?;
    Ok(result.rows_affected() == 1)
}
