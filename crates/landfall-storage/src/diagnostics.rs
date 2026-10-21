//! Append-only persistence of versioned diagnostics and evidence links.
//!
//! Each trace re-evaluation is recorded as a `reporting.diagnostic_evaluations`
//! row that lists the diagnostics and recommendations it produced. Rows are never
//! updated or deleted; readers serve only the newest evaluation of a trace.

#![allow(missing_docs)]

use sqlx::{PgPool, Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::recommendations::{RecommendationWrite, insert_recommendation};

#[derive(Debug, Clone)]
pub struct DiagnosticWrite {
    pub diagnostic_id: Uuid,
    pub trace_id: Uuid,
    pub rule_id: String,
    pub rule_set_version: String,
    pub claim_key: String,
    pub certainty: String,
    pub unknown_reason: Option<String>,
    pub created_at: OffsetDateTime,
    pub evidence_event_ids: Vec<Uuid>,
}

/// Complete output of one deterministic evaluation of a trace snapshot.
#[derive(Debug, Clone)]
pub struct TraceEvaluationWrite {
    pub evaluation_id: Uuid,
    pub trace_id: Uuid,
    /// Number of immutable events in the evaluated snapshot; larger snapshots win.
    pub input_event_count: i64,
    pub evaluated_at: OffsetDateTime,
    pub diagnostics: Vec<DiagnosticWrite>,
    pub recommendations: Vec<RecommendationWrite>,
}

/// One diagnostic produced by the newest evaluation of a trace.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CurrentDiagnostic {
    pub diagnostic_id: Uuid,
    pub rule_id: String,
    pub claim_key: String,
    pub certainty: String,
    pub unknown_reason: Option<String>,
    /// Raw events the rule evaluated, in event-ID order.
    pub evidence_event_ids: Vec<Uuid>,
}

/// One recommendation produced by the newest evaluation of a trace.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CurrentRecommendation {
    pub recommendation_id: Uuid,
    pub recommendation_key: String,
    pub rule_set_version: String,
}

/// Appends diagnostics and evidence links in one transaction; prior versions remain queryable.
pub async fn append_diagnostics(
    pool: &PgPool,
    diagnostics: &[DiagnosticWrite],
) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    for diagnostic in diagnostics {
        insert_diagnostic(&mut tx, diagnostic).await?;
    }
    tx.commit().await
}

/// Records one trace evaluation with all of its findings and advice atomically.
pub async fn record_trace_evaluation(
    pool: &PgPool,
    evaluation: &TraceEvaluationWrite,
) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    sqlx::query("INSERT INTO reporting.diagnostic_evaluations (evaluation_id, trace_id, input_event_count, evaluated_at) VALUES ($1,$2,$3,$4)")
        .bind(evaluation.evaluation_id).bind(evaluation.trace_id).bind(evaluation.input_event_count).bind(evaluation.evaluated_at).execute(&mut *tx).await?;
    for diagnostic in &evaluation.diagnostics {
        insert_diagnostic(&mut tx, diagnostic).await?;
        sqlx::query("INSERT INTO reporting.diagnostic_evaluation_diagnostics (evaluation_id, diagnostic_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(evaluation.evaluation_id).bind(diagnostic.diagnostic_id).execute(&mut *tx).await?;
    }
    for recommendation in &evaluation.recommendations {
        insert_recommendation(&mut tx, recommendation).await?;
        sqlx::query("INSERT INTO reporting.diagnostic_evaluation_recommendations (evaluation_id, recommendation_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(evaluation.evaluation_id).bind(recommendation.recommendation_id).execute(&mut *tx).await?;
    }
    tx.commit().await
}

const LATEST_EVALUATION: &str = "SELECT e.evaluation_id FROM reporting.diagnostic_evaluations e JOIN reporting.traces t ON t.trace_id = e.trace_id WHERE e.trace_id = $1 AND ($2::uuid IS NULL OR t.project_id = $2) ORDER BY e.input_event_count DESC, e.evaluated_at DESC, e.evaluation_id DESC LIMIT 1";

/// Diagnostics of the newest evaluation, optionally restricted to one project.
pub async fn current_trace_diagnostics(
    pool: &PgPool,
    trace_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<Vec<CurrentDiagnostic>, sqlx::Error> {
    let rows = sqlx::query(&format!("WITH latest AS ({LATEST_EVALUATION}) SELECT d.diagnostic_id, d.rule_id, d.claim_key, d.certainty, d.unknown_reason, ARRAY(SELECT de.event_id FROM reporting.diagnostic_evidence de WHERE de.diagnostic_id = d.diagnostic_id ORDER BY de.event_id) AS evidence_event_ids FROM latest JOIN reporting.diagnostic_evaluation_diagnostics m ON m.evaluation_id = latest.evaluation_id JOIN reporting.diagnostics d ON d.diagnostic_id = m.diagnostic_id ORDER BY d.created_at DESC, d.diagnostic_id"))
        .bind(trace_id).bind(project_id).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(CurrentDiagnostic {
                diagnostic_id: row.try_get("diagnostic_id")?,
                rule_id: row.try_get("rule_id")?,
                claim_key: row.try_get("claim_key")?,
                certainty: row.try_get("certainty")?,
                unknown_reason: row.try_get("unknown_reason")?,
                evidence_event_ids: row.try_get("evidence_event_ids")?,
            })
        })
        .collect()
}

/// Recommendations of the newest evaluation, optionally restricted to one project.
pub async fn current_trace_recommendations(
    pool: &PgPool,
    trace_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<Vec<CurrentRecommendation>, sqlx::Error> {
    let rows = sqlx::query(&format!("WITH latest AS ({LATEST_EVALUATION}) SELECT r.recommendation_id, r.recommendation_key, r.rule_set_version FROM latest JOIN reporting.diagnostic_evaluation_recommendations m ON m.evaluation_id = latest.evaluation_id JOIN reporting.recommendations r ON r.recommendation_id = m.recommendation_id ORDER BY r.created_at DESC, r.recommendation_id"))
        .bind(trace_id).bind(project_id).fetch_all(pool).await?;
    rows.into_iter()
        .map(|row| {
            Ok(CurrentRecommendation {
                recommendation_id: row.try_get("recommendation_id")?,
                recommendation_key: row.try_get("recommendation_key")?,
                rule_set_version: row.try_get("rule_set_version")?,
            })
        })
        .collect()
}

async fn insert_diagnostic(
    tx: &mut Transaction<'_, Postgres>,
    diagnostic: &DiagnosticWrite,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO reporting.diagnostics (diagnostic_id, trace_id, rule_id, rule_set_version, claim_key, certainty, unknown_reason, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT (diagnostic_id) DO NOTHING")
        .bind(diagnostic.diagnostic_id).bind(diagnostic.trace_id).bind(&diagnostic.rule_id).bind(&diagnostic.rule_set_version).bind(&diagnostic.claim_key).bind(&diagnostic.certainty).bind(&diagnostic.unknown_reason).bind(diagnostic.created_at).execute(&mut **tx).await?;
    for event_id in &diagnostic.evidence_event_ids {
        sqlx::query("INSERT INTO reporting.diagnostic_evidence (diagnostic_id, event_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
            .bind(diagnostic.diagnostic_id).bind(event_id).execute(&mut **tx).await?;
    }
    Ok(())
}
