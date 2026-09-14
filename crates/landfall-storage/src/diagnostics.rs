//! Append-only persistence of versioned diagnostics and evidence links.

#![allow(missing_docs)]

use sqlx::{PgPool, Postgres, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DiagnosticWrite {
    pub diagnostic_id: Uuid,
    pub trace_id: Uuid,
    pub rule_id: String,
    pub rule_set_version: String,
    pub claim_key: String,
    pub certainty: String,
    pub created_at: OffsetDateTime,
    pub evidence_event_ids: Vec<Uuid>,
}

/// Appends diagnostics and evidence links in one transaction; prior versions remain queryable.
pub async fn append_diagnostics(
    pool: &PgPool,
    diagnostics: &[DiagnosticWrite],
) -> Result<(), sqlx::Error> {
    let mut tx: Transaction<'_, Postgres> = pool.begin().await?;
    for diagnostic in diagnostics {
        sqlx::query("INSERT INTO reporting.diagnostics (diagnostic_id, trace_id, rule_id, rule_set_version, claim_key, certainty, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (diagnostic_id) DO NOTHING")
            .bind(diagnostic.diagnostic_id).bind(diagnostic.trace_id).bind(&diagnostic.rule_id).bind(&diagnostic.rule_set_version).bind(&diagnostic.claim_key).bind(&diagnostic.certainty).bind(diagnostic.created_at).execute(&mut *tx).await?;
        for event_id in &diagnostic.evidence_event_ids {
            sqlx::query("INSERT INTO reporting.diagnostic_evidence (diagnostic_id, event_id) VALUES ($1,$2) ON CONFLICT DO NOTHING")
                .bind(diagnostic.diagnostic_id).bind(event_id).execute(&mut *tx).await?;
        }
    }
    tx.commit().await
}
