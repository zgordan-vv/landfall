//! Comparison roles come from explicit configuration, never from recency.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use landfall_storage::{
    IngestEvent, ensure_raw_event_partition, ingest_atomically, reset_test_data, run_migrations,
    seed_control_plane,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    pool: sqlx::PgPool,
    project_id: Uuid,
    baseline: Uuid,
    candidate: Uuid,
}

async fn environment(pool: &sqlx::PgPool, project_id: Uuid, name: &str) -> Uuid {
    let environment_id = Uuid::now_v7();
    sqlx::query("INSERT INTO control.environments (environment_id, project_id, name, cluster, description) VALUES ($1, $2, $3, 'devnet', $4)")
        .bind(environment_id)
        .bind(project_id)
        .bind(name)
        .bind(format!("{name} settings"))
        .execute(pool)
        .await
        .unwrap();
    environment_id
}

/// One trace: its projected states plus the raw events the comparison reads.
async fn trace(
    fixture: &Fixture,
    environment_id: Uuid,
    started: OffsetDateTime,
    landing_state: &str,
    events: &[(&str, i64, Value)],
) {
    let trace_id = Uuid::now_v7();
    let execution_state = if landing_state == "finalized" {
        "success"
    } else {
        "unknown"
    };
    sqlx::query("INSERT INTO reporting.traces (trace_id, project_id, environment_id, landing_state, execution_state) VALUES ($1, $2, $3, $4, $5)")
        .bind(trace_id)
        .bind(fixture.project_id)
        .bind(environment_id)
        .bind(landing_state)
        .bind(execution_state)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let mut writes = Vec::new();
    for (event_type, offset_ms, attributes) in
        std::iter::once(&("solana.trace.created", 0, json!({}))).chain(events)
    {
        let occurred_at = started + Duration::milliseconds(*offset_ms);
        ensure_raw_event_partition(&fixture.pool, occurred_at.date())
            .await
            .unwrap();
        let payload = json!({ "event_type": event_type, "attributes": attributes });
        writes.push(IngestEvent {
            event_id: Uuid::now_v7(),
            project_id: fixture.project_id,
            environment_id,
            trace_id: Some(trace_id),
            event_type: (*event_type).to_owned(),
            occurred_at,
            payload_hash: Sha256::digest(serde_json::to_vec(&payload).unwrap()).to_vec(),
            payload,
        });
    }
    ingest_atomically(&fixture.pool, Uuid::now_v7(), &writes)
        .await
        .unwrap();
}

fn submitted() -> (&'static str, i64, Value) {
    ("solana.submission.started", 100, json!({}))
}

fn confirmed(offset_ms: i64) -> (&'static str, i64, Value) {
    (
        "solana.status.observed",
        offset_ms,
        json!({ "source_result": "found", "commitment": "confirmed" }),
    )
}

fn fee(lamports: &str) -> (&'static str, i64, Value) {
    (
        "solana.execution.enriched",
        5_000,
        json!({ "fee_lamports": lamports }),
    )
}

async fn get(fixture: &Fixture, query: &str) -> (StatusCode, Value) {
    let state = super::AppState {
        ready: true,
        pool: Some(fixture.pool.clone()),
        public_demo_project_id: Some(fixture.project_id),
        ..super::AppState::default()
    };
    let response = super::router(state)
        .oneshot(
            Request::get(format!("/demo/v1/comparison{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1 << 20)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

fn metric<'a>(report: &'a Value, key: &str) -> &'a Value {
    report["metrics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|metric| metric["key"] == key)
        .unwrap()
}

/// Exercises `/demo/v1/comparison` against PostgreSQL when
/// `LANDFALL_TEST_DATABASE_URL` points at a disposable database (its data is reset).
#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn comparison_uses_designated_roles_and_windowed_evidence() {
    let Ok(url) = std::env::var("LANDFALL_TEST_DATABASE_URL") else {
        return;
    };
    let _guard = super::DATABASE_TEST_LOCK.lock().await;
    let pool = sqlx::PgPool::connect(&url).await.unwrap();
    run_migrations(&pool).await.unwrap();
    reset_test_data(&pool).await.unwrap();
    let seed = seed_control_plane(&pool).await.unwrap();
    let fixture = Fixture {
        project_id: seed.project_id,
        baseline: environment(&pool, seed.project_id, "baseline").await,
        candidate: environment(&pool, seed.project_id, "candidate").await,
        pool,
    };
    let now = OffsetDateTime::now_utc() - Duration::minutes(5);
    trace(
        &fixture,
        fixture.baseline,
        now,
        "finalized",
        &[submitted(), confirmed(2_100), fee("5000")],
    )
    .await;
    trace(
        &fixture,
        fixture.baseline,
        now,
        "expired",
        &[
            submitted(),
            ("solana.submission.retry_scheduled", 200, json!({})),
            ("solana.submission.started", 300, json!({})),
        ],
    )
    .await;
    // Outside the default seven-day window.
    trace(
        &fixture,
        fixture.baseline,
        now - Duration::days(10),
        "expired",
        &[submitted()],
    )
    .await;
    // Projected last, so a recency rule would make it the candidate.
    trace(
        &fixture,
        fixture.candidate,
        now,
        "finalized",
        &[submitted(), confirmed(1_100), fee("7000")],
    )
    .await;
    sqlx::query("UPDATE reporting.traces SET updated_at = now() + interval '1 minute' WHERE environment_id = $1")
        .bind(fixture.baseline)
        .execute(&fixture.pool)
        .await
        .unwrap();

    let (status, report) = get(&fixture, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["status"], "not_configured");
    assert_eq!(report["environments"].as_array().unwrap().len(), 3);
    assert_eq!(report["metrics"], json!([]));

    sqlx::query("INSERT INTO control.comparison_cohorts (project_id, baseline_environment_id, candidate_environment_id) VALUES ($1, $2, $3)")
        .bind(fixture.project_id)
        .bind(fixture.baseline)
        .bind(fixture.candidate)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let (status, report) = get(&fixture, "").await;
    assert_eq!(status, StatusCode::OK, "{report}");
    assert_eq!(report["status"], "ready");
    assert_eq!(report["role_source"], "project_setting");
    assert_eq!(report["window"]["hours"], 168);
    assert_eq!(
        report["baseline"]["environment"]["environment_id"],
        fixture.baseline.to_string()
    );
    assert_eq!(
        report["baseline"]["environment"]["description"],
        "baseline settings"
    );
    assert_eq!(report["baseline"]["traces"], 2);
    assert_eq!(report["candidate"]["traces"], 1);
    assert_eq!(report["sufficient_sample"], false);

    let landing = metric(&report, "landing_rate");
    assert_eq!(landing["baseline"]["value"], 0.5);
    assert_eq!(landing["candidate"]["value"], 1.0);
    assert_eq!(landing["outcome"], "improved");
    assert_eq!(metric(&report, "expiry_rate")["outcome"], "improved");
    assert_eq!(metric(&report, "retry_rate")["baseline"]["numerator"], 1);
    let latency = metric(&report, "median_confirmation_ms");
    assert_eq!(latency["baseline"]["value"], 2_000.0);
    assert_eq!(latency["candidate"]["value"], 1_000.0);
    assert_eq!(latency["outcome"], "improved");
    assert_eq!(metric(&report, "median_fee_lamports")["outcome"], "changed");

    // Explicit roles override the setting and may swap it.
    let (status, report) = get(
        &fixture,
        &format!(
            "?baseline_environment_id={}&candidate_environment_id={}&window_hours=720",
            fixture.candidate, fixture.baseline
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(report["role_source"], "query");
    assert_eq!(report["candidate"]["traces"], 3);
    assert_eq!(metric(&report, "landing_rate")["outcome"], "regressed");

    let (status, report) = get(
        &fixture,
        &format!("?baseline_environment_id={}", fixture.baseline),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(report["code"], "incomplete_cohort_roles");

    // Environments of another project are neither readable nor designatable.
    let other_project = Uuid::now_v7();
    sqlx::query("INSERT INTO control.projects (project_id, name) VALUES ($1, 'other')")
        .bind(other_project)
        .execute(&fixture.pool)
        .await
        .unwrap();
    let foreign = environment(&fixture.pool, other_project, "foreign").await;
    let (status, _) = get(
        &fixture,
        &format!(
            "?baseline_environment_id={}&candidate_environment_id={foreign}",
            fixture.baseline
        ),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let designated = sqlx::query(
        "UPDATE control.comparison_cohorts SET candidate_environment_id = $2 WHERE project_id = $1",
    )
    .bind(fixture.project_id)
    .bind(foreign)
    .execute(&fixture.pool)
    .await;
    assert!(designated.is_err(), "foreign environment was accepted");

    reset_test_data(&fixture.pool).await.unwrap();
}
