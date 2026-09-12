//! Canonical raw-event loading for projection.
#![allow(missing_docs)]

use serde_json::Value;
use sqlx::{FromRow, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

/// Raw event row supplied to the deterministic projector.
#[derive(Debug, Clone, FromRow)]
pub struct RawEventRow {
    pub event_id: Uuid,
    pub project_id: Uuid,
    pub environment_id: Uuid,
    pub trace_id: Option<Uuid>,
    pub event_type: String,
    pub occurred_at: OffsetDateTime,
    pub received_at: OffsetDateTime,
    pub payload: Value,
}

/// RPC target extracted from durable submission evidence.
#[derive(Debug, Clone, FromRow)]
pub struct ObservationTarget {
    pub trace_id: Uuid,
    pub route_id: Uuid,
    pub endpoint: String,
    pub signature: String,
}

/// Loads the most recent accepted signature and its enabled route.
pub async fn load_observation_target(
    pool: &PgPool,
    trace_id: Uuid,
) -> Result<Option<ObservationTarget>, sqlx::Error> {
    sqlx::query_as::<_, ObservationTarget>("SELECT e.trace_id, c.route_id, c.endpoint, e.payload #>> '{attributes,signature}' AS signature FROM telemetry.raw_events e JOIN control.routes c ON c.route_id = (e.payload #>> '{attributes,route_id}')::uuid WHERE e.trace_id = $1 AND e.event_type = 'solana.submission.completed' AND c.enabled AND e.payload #>> '{attributes,signature}' IS NOT NULL ORDER BY e.occurred_at DESC, e.received_at DESC LIMIT 1").bind(trace_id).fetch_optional(pool).await
}

/// Loads events for one trace within an explicit half-open time range.
pub async fn load_events_for_trace(
    pool: &PgPool,
    project_id: Uuid,
    environment_id: Uuid,
    trace_id: Uuid,
    from: OffsetDateTime,
    until: OffsetDateTime,
) -> Result<Vec<RawEventRow>, sqlx::Error> {
    sqlx::query_as::<_, RawEventRow>("SELECT event_id, project_id, environment_id, trace_id, event_type, occurred_at, received_at, payload FROM telemetry.raw_events WHERE project_id = $1 AND environment_id = $2 AND trace_id = $3 AND occurred_at >= $4 AND occurred_at < $5 ORDER BY occurred_at, received_at, event_id")
        .bind(project_id).bind(environment_id).bind(trace_id).bind(from).bind(until).fetch_all(pool).await
}
