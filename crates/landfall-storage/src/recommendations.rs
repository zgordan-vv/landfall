//! Append-only persistence of recommendations and their evidence graph.

#![allow(missing_docs)]

use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RecommendationWrite {
    pub recommendation_id: Uuid,
    pub trace_id: Uuid,
    pub recommendation_key: String,
    pub rule_set_version: String,
    pub created_at: OffsetDateTime,
    pub diagnostic_ids: Vec<Uuid>,
    pub evidence_event_ids: Vec<Uuid>,
}

/// Appends recommendations and all graph links atomically.
pub async fn append_recommendations(
    pool: &PgPool,
    recommendations: &[RecommendationWrite],
) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    for recommendation in recommendations {
        sqlx::query("INSERT INTO reporting.recommendations (recommendation_id, trace_id, recommendation_key, rule_set_version, created_at) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (recommendation_id) DO NOTHING")
            .bind(recommendation.recommendation_id).bind(recommendation.trace_id).bind(&recommendation.recommendation_key).bind(&recommendation.rule_set_version).bind(recommendation.created_at).execute(&mut *tx).await?;
        for diagnostic_id in &recommendation.diagnostic_ids {
            sqlx::query("INSERT INTO reporting.recommendation_diagnostics (recommendation_id, diagnostic_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
                .bind(recommendation.recommendation_id).bind(diagnostic_id).execute(&mut *tx).await?;
        }
        for event_id in &recommendation.evidence_event_ids {
            sqlx::query("INSERT INTO reporting.recommendation_evidence (recommendation_id, event_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
                .bind(recommendation.recommendation_id).bind(event_id).execute(&mut *tx).await?;
        }
    }
    tx.commit().await
}
