//! Eligibility-gated observation job enqueueing.

use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
/// An observation job claimed by one worker instance.
pub struct ClaimedObservationJob {
    /// Durable job identity.
    pub job_id: Uuid,
    /// Trace to observe.
    pub trace_id: Uuid,
    /// Attempt number after this claim.
    pub attempts: i32,
}

/// Durable result of a failed observation attempt.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ObservationRetryOutcome {
    /// The job remains eligible for a later attempt.
    RetryScheduled,
    /// The retry budget was exhausted and the job requires operator attention.
    DeadLettered,
}

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

/// Claims one ready observation job with a renewable lease.
pub async fn claim_observation_job(
    pool: &PgPool,
    worker_id: &str,
    lease_seconds: i64,
) -> Result<Option<ClaimedObservationJob>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, Value, i32)>("SELECT job_id, payload, attempts FROM work.jobs WHERE job_type = 'observe_trace' AND status = 'ready' AND available_at <= now() ORDER BY available_at, job_id FOR UPDATE SKIP LOCKED LIMIT 1")
        .fetch_optional(&mut *tx).await?;
    let Some((job_id, payload, attempts)) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let Some(trace_id) = payload
        .get("trace_id")
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    else {
        return Ok(None);
    };
    sqlx::query("UPDATE work.jobs SET status = 'running', locked_by = $1, locked_until = now() + make_interval(secs => $2), attempts = attempts + 1 WHERE job_id = $3")
        .bind(worker_id).bind(lease_seconds as f64).bind(job_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Some(ClaimedObservationJob {
        job_id,
        trace_id,
        attempts: attempts + 1,
    }))
}

/// Marks a claimed observation job complete and removes its active lease.
pub async fn complete_observation_job(pool: &PgPool, job_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE work.jobs SET status = 'completed', locked_by = NULL, locked_until = NULL, completed_at = now() WHERE job_id = $1 AND status = 'running'").bind(job_id).execute(pool).await?;
    Ok(())
}

/// Returns a failed observation job to the queue with bounded backoff.
pub async fn retry_observation_job(
    pool: &PgPool,
    job_id: Uuid,
    error: &str,
    delay_seconds: i64,
) -> Result<ObservationRetryOutcome, sqlx::Error> {
    let outcome = sqlx::query_scalar::<_, String>("UPDATE work.jobs SET status = CASE WHEN attempts >= 10 THEN 'dead_letter' ELSE 'ready' END, locked_by = NULL, locked_until = NULL, last_error = $2, available_at = CASE WHEN attempts >= 10 THEN available_at ELSE now() + make_interval(secs => $3) END WHERE job_id = $1 AND status = 'running' RETURNING status")
        .bind(job_id)
        .bind(error)
        .bind(delay_seconds as f64)
        .fetch_optional(pool)
        .await?;
    Ok(match outcome.as_deref() {
        Some("dead_letter") => ObservationRetryOutcome::DeadLettered,
        _ => ObservationRetryOutcome::RetryScheduled,
    })
}

/// Requeues observation jobs whose worker lease expired after a crash.
pub async fn reclaim_expired_observation_jobs(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE work.jobs SET status = 'ready', locked_by = NULL, locked_until = NULL, available_at = now() WHERE job_type = 'observe_trace' AND status = 'running' AND locked_until < now()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}
