//! Append-only recommendation disposition read/write semantics.

use serde::{Deserialize, Serialize};

/// User decision recorded for an advisory recommendation.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecommendationDisposition {
    Accepted,
    Rejected,
    Implemented,
    NotApplicable,
}

/// One immutable audit entry. A new decision appends another entry.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct DispositionRecord {
    pub disposition_id: String,
    pub recommendation_id: String,
    pub disposition: RecommendationDisposition,
    pub reason: Option<String>,
    pub recorded_at: String,
    pub actor: String,
}

/// Validation failure before persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispositionError {
    EmptyActor,
    EmptyTimestamp,
    ReasonTooLong,
    DuplicateDispositionId,
}

impl std::fmt::Display for DispositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmptyActor => "actor must not be empty",
            Self::EmptyTimestamp => "recorded_at must not be empty",
            Self::ReasonTooLong => "reason exceeds the 1,000-byte limit",
            Self::DuplicateDispositionId => "disposition_id already exists in history",
        })
    }
}

impl std::error::Error for DispositionError {}

/// Validates and appends one disposition without rewriting prior history.
pub fn append_disposition(
    history: &mut Vec<DispositionRecord>,
    record: DispositionRecord,
) -> Result<(), DispositionError> {
    if record.actor.trim().is_empty() {
        return Err(DispositionError::EmptyActor);
    }
    if record.recorded_at.trim().is_empty() {
        return Err(DispositionError::EmptyTimestamp);
    }
    if record
        .reason
        .as_deref()
        .is_some_and(|reason| reason.len() > 1_000)
    {
        return Err(DispositionError::ReasonTooLong);
    }
    if history
        .iter()
        .any(|entry| entry.disposition_id == record.disposition_id)
    {
        return Err(DispositionError::DuplicateDispositionId);
    }
    history.push(record);
    Ok(())
}

/// Returns the most recently appended decision for a recommendation.
#[must_use]
pub fn latest_disposition<'a>(
    history: &'a [DispositionRecord],
    recommendation_id: &str,
) -> Option<&'a DispositionRecord> {
    history
        .iter()
        .rev()
        .find(|entry| entry.recommendation_id == recommendation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        id: &str,
        recommendation_id: &str,
        disposition: RecommendationDisposition,
    ) -> DispositionRecord {
        DispositionRecord {
            disposition_id: id.into(),
            recommendation_id: recommendation_id.into(),
            disposition,
            reason: Some("reviewed".into()),
            recorded_at: "2026-01-01T00:00:00Z".into(),
            actor: "operator".into(),
        }
    }

    #[test]
    fn appends_history_and_returns_latest() {
        let recommendation_id = "recommendation-1";
        let first_id = "disposition-1";
        let second_id = "disposition-2";
        let mut history = Vec::new();
        append_disposition(
            &mut history,
            record(
                first_id,
                recommendation_id,
                RecommendationDisposition::Accepted,
            ),
        )
        .unwrap();
        append_disposition(
            &mut history,
            record(
                second_id,
                recommendation_id,
                RecommendationDisposition::Implemented,
            ),
        )
        .unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(
            latest_disposition(&history, recommendation_id)
                .unwrap()
                .disposition,
            RecommendationDisposition::Implemented
        );
    }

    #[test]
    fn duplicate_entries_are_rejected() {
        let recommendation_id = "recommendation-1";
        let disposition_id = "disposition-1";
        let mut history = vec![record(
            disposition_id,
            recommendation_id,
            RecommendationDisposition::Rejected,
        )];
        assert_eq!(
            append_disposition(
                &mut history,
                record(
                    disposition_id,
                    recommendation_id,
                    RecommendationDisposition::Accepted
                )
            ),
            Err(DispositionError::DuplicateDispositionId)
        );
    }
}
