//! Atomic ingestion transaction boundary.
#![allow(missing_docs)]

use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct IngestEvent {
    pub event_id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Uuid,
    pub trace_id: Option<Uuid>,
    pub event_type: String,
    pub occurred_at: OffsetDateTime,
    pub payload: Value,
    pub payload_hash: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct IngestOutcome {
    pub inserted: usize,
    pub duplicates: usize,
}

/// Claims dedup identities, inserts raw events, and enqueues projection work atomically.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub async fn ingest_atomically(
    pool: &PgPool,
    batch_id: Uuid,
    events: &[IngestEvent],
) -> Result<IngestOutcome, sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    sqlx::query("INSERT INTO telemetry.ingest_batches (batch_id, event_count) VALUES ($1, $2)")
        .bind(batch_id)
        .bind(events.len() as i32)
        .execute(&mut *tx)
        .await?;
    let mut inserted = 0;
    let mut duplicates = 0;
    for event in events {
        let claimed = sqlx::query("INSERT INTO telemetry.event_dedup (event_id, batch_id, payload_hash) VALUES ($1, $2, $3) ON CONFLICT (event_id) DO NOTHING")
            .bind(event.event_id).bind(batch_id).bind(&event.payload_hash).execute(&mut *tx).await?.rows_affected() == 1;
        if !claimed {
            duplicates += 1;
            continue;
        }
        sqlx::query("INSERT INTO telemetry.raw_events (event_id, project_id, environment_id, trace_id, event_type, occurred_at, payload) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(event.event_id).bind(event.project_id).bind(event.environment_id).bind(event.trace_id).bind(&event.event_type).bind(event.occurred_at).bind(&event.payload).execute(&mut *tx).await?;
        if let Some(trace_id) = event.trace_id {
            sqlx::query("INSERT INTO work.jobs (job_id, job_type, dedupe_key, payload) VALUES ($1, 'project_trace', $2, jsonb_build_object('trace_id', $3)) ON CONFLICT (job_type, dedupe_key) WHERE status IN ('ready', 'running') DO NOTHING")
                .bind(Uuid::now_v7()).bind(trace_id.to_string()).bind(trace_id).execute(&mut *tx).await?;
        }
        inserted += 1;
    }
    tx.commit().await?;
    Ok(IngestOutcome {
        inserted,
        duplicates,
    })
}
