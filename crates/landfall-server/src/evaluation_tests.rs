//! Trace re-evaluation must replace, not accumulate, the findings readers see.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::str::FromStr;

use landfall_core::ordering::{CollectedEvent, OrderingConfig, canonical_order};
use landfall_protocol::{UtcTimestamp, WireEvent};
use landfall_storage::{
    DiagnosticWrite, IngestEvent, current_trace_diagnostics, current_trace_recommendations,
    ensure_raw_event_partition, ingest_atomically, reset_test_data, run_migrations,
    seed_control_plane,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const MISSING_OBSERVER_COVERAGE: &str = "missing_observer_coverage";

fn fixture(json: &str) -> Vec<Value> {
    serde_json::from_str::<Value>(json).unwrap()["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|event| event.get("trace_id").is_some())
        .cloned()
        .collect()
}

/// Events up to an accepted submission, then the confirmation and observer evidence.
fn submitted_then_observed() -> (Vec<Value>, Vec<Value>) {
    let mut submitted = true;
    fixture(include_str!(
        "../../../fixtures/protocol/v1/valid/all-event-types.batch.json"
    ))
    .into_iter()
    .partition(|event| {
        let in_first_batch = submitted;
        submitted &= event["event_type"] != "solana.submission.completed";
        in_first_batch
    })
}

/// An expired trace, then a second `not_found` observation from the server observer.
fn expired_then_second_not_found() -> (Vec<Value>, Vec<Value>) {
    let first = fixture(include_str!(
        "../../../fixtures/protocol/v1/valid/incidents/expiry-without-inclusion.batch.json"
    ));
    let mut second = first
        .iter()
        .find(|event| event["event_type"] == "solana.status.observed")
        .cloned()
        .unwrap();
    second["event_id"] = Value::from("0198ef10-0005-7000-8000-000000000013");
    second["occurred_at"] = Value::from("2026-08-29T12:00:01.6Z");
    second["attributes"]["block_height"] = Value::from("102");
    (first, vec![second])
}

fn evaluate(events: &[Value]) -> Vec<DiagnosticWrite> {
    let received_at = UtcTimestamp::from_str("2026-08-29T12:01:00Z").unwrap();
    let ordered = canonical_order(
        events.iter().map(|event| {
            CollectedEvent::new(
                serde_json::from_value::<WireEvent>(event.clone()).unwrap(),
                received_at,
            )
        }),
        OrderingConfig::default(),
    )
    .unwrap();
    let projection = landfall_core::reducer::reduce_trace(&ordered).unwrap();
    let trace_id = projection.trace().id().into_uuid();
    super::evaluate_trace_findings(
        trace_id,
        &ordered,
        &projection,
        time::OffsetDateTime::UNIX_EPOCH,
    )
    .unwrap()
    .0
}

fn reasons(diagnostics: &[DiagnosticWrite]) -> Vec<&str> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.unknown_reason.as_deref())
        .collect()
}

#[test]
fn partial_snapshot_reports_observer_gap_that_full_snapshot_does_not() {
    let (submitted, observed) = submitted_then_observed();
    let partial = evaluate(&submitted);
    assert!(reasons(&partial).contains(&MISSING_OBSERVER_COVERAGE));

    let full = evaluate(&[submitted, observed].concat());
    assert!(!reasons(&full).contains(&MISSING_OBSERVER_COVERAGE));
}

#[test]
fn every_unknown_finding_keeps_its_own_reason() {
    let (submitted, _) = submitted_then_observed();
    let diagnostics = evaluate(&submitted);
    let unknown = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.certainty == "unknown")
        .collect::<Vec<_>>();
    assert!(!unknown.is_empty());
    assert!(
        unknown
            .iter()
            .all(|diagnostic| diagnostic.unknown_reason.is_some())
    );
    let mut ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.diagnostic_id)
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), diagnostics.len());
}

#[test]
fn stored_diagnostic_ids_are_deterministic() {
    let (submitted, _) = submitted_then_observed();
    let ids = |diagnostics: Vec<DiagnosticWrite>| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.diagnostic_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(evaluate(&submitted)), ids(evaluate(&submitted)));
}

async fn ingest(pool: &sqlx::PgPool, project_id: Uuid, environment_id: Uuid, events: &[Value]) {
    let mut writes = Vec::new();
    for event in events {
        let mut event = event.clone();
        event["project_id"] = Value::from(project_id.to_string());
        event["environment_id"] = Value::from(environment_id.to_string());
        let occurred_at = time::OffsetDateTime::parse(
            event["occurred_at"].as_str().unwrap(),
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        ensure_raw_event_partition(pool, occurred_at.date())
            .await
            .unwrap();
        let payload_hash = Sha256::digest(serde_json::to_vec(&event).unwrap()).to_vec();
        writes.push(IngestEvent {
            event_id: Uuid::parse_str(event["event_id"].as_str().unwrap()).unwrap(),
            project_id,
            environment_id,
            trace_id: Some(Uuid::parse_str(event["trace_id"].as_str().unwrap()).unwrap()),
            event_type: event["event_type"].as_str().unwrap().to_owned(),
            occurred_at,
            payload: event,
            payload_hash,
        });
    }
    for write in &writes {
        sqlx::query("INSERT INTO reporting.traces (trace_id, project_id, environment_id) VALUES ($1, $2, $3) ON CONFLICT (trace_id) DO NOTHING")
            .bind(write.trace_id).bind(project_id).bind(environment_id).execute(pool).await.unwrap();
    }
    ingest_atomically(pool, Uuid::now_v7(), &writes)
        .await
        .unwrap();
}

async fn ingest_in_batches(
    pool: &sqlx::PgPool,
    batches: (Vec<Value>, Vec<Value>),
) -> Vec<landfall_storage::CurrentDiagnostic> {
    reset_test_data(pool).await.unwrap();
    let seed = seed_control_plane(pool).await.unwrap();
    let trace_id = Uuid::parse_str(batches.0[0]["trace_id"].as_str().unwrap()).unwrap();
    for batch in [batches.0, batches.1] {
        ingest(pool, seed.project_id, seed.environment_id, &batch).await;
        super::refresh_trace_projection(pool, seed.project_id, seed.environment_id, trace_id)
            .await
            .unwrap();
    }
    let recommendations = current_trace_recommendations(pool, trace_id, Some(seed.project_id))
        .await
        .unwrap();
    let mut recommendation_ids = recommendations
        .iter()
        .map(|recommendation| recommendation.recommendation_id)
        .collect::<Vec<_>>();
    recommendation_ids.dedup();
    assert_eq!(recommendation_ids.len(), recommendations.len());
    current_trace_diagnostics(pool, trace_id, Some(seed.project_id))
        .await
        .unwrap()
}

/// Exercises the storage readers against PostgreSQL when
/// `LANDFALL_TEST_DATABASE_URL` points at a disposable database (its data is reset).
#[tokio::test]
async fn readers_serve_only_the_latest_evaluation() {
    let Ok(url) = std::env::var("LANDFALL_TEST_DATABASE_URL") else {
        return;
    };
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    run_migrations(&pool).await.unwrap();

    let current = ingest_in_batches(&pool, submitted_then_observed()).await;
    assert!(
        current
            .iter()
            .all(|row| row.unknown_reason.as_deref() != Some(MISSING_OBSERVER_COVERAGE)),
        "stale partial-snapshot finding is still served: {current:?}"
    );
    let total: i64 =
        sqlx::query_scalar("SELECT count(*) FROM reporting.diagnostics WHERE unknown_reason = $1")
            .bind(MISSING_OBSERVER_COVERAGE)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(total, 1, "history must keep the superseded finding");

    let current = ingest_in_batches(&pool, expired_then_second_not_found()).await;
    assert_eq!(
        current
            .iter()
            .filter(|row| row.rule_id == "RULE-EXP-001")
            .count(),
        1,
        "{current:?}"
    );
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM reporting.diagnostics WHERE rule_id = 'RULE-EXP-001'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        total, 2,
        "each not_found snapshot keeps its own history row"
    );

    reset_test_data(&pool).await.unwrap();
}
