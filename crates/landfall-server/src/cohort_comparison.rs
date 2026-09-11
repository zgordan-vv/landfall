//! Descriptive, bounded comparison model for two completed cohorts.

use serde::Serialize;
use utoipa::ToSchema;

/// Input metrics for one frozen cohort selection.
#[derive(Debug, Clone, Copy)]
pub struct CohortInput {
    pub sample_size: u64,
    pub completed_observations: u64,
    pub successful_observations: u64,
    pub missing_observations: u64,
    pub metric_definition: &'static str,
}

/// Why a comparison was rejected before calculating misleading deltas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonError {
    InsufficientCompletedObservations,
    InvalidCounts,
}

impl std::fmt::Display for ComparisonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InsufficientCompletedObservations => {
                "both cohorts must have the minimum completed observation count"
            }
            Self::InvalidCounts => "cohort counts are internally inconsistent",
        })
    }
}

impl std::error::Error for ComparisonError {}

/// Descriptive comparison of candidate metrics against a baseline cohort.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq)]
pub struct CohortComparison {
    pub baseline_rate: Option<f64>,
    pub candidate_rate: Option<f64>,
    pub absolute_change: Option<f64>,
    pub relative_change: Option<f64>,
    pub baseline_sample_size: u64,
    pub candidate_sample_size: u64,
    pub baseline_missing_data_rate: f64,
    pub candidate_missing_data_rate: f64,
    pub small_sample_warning: bool,
    pub metric_definition: String,
}

/// Compares two cohorts using completed observations only.
pub fn compare_cohorts(
    baseline: CohortInput,
    candidate: CohortInput,
    minimum_completed: u64,
    small_sample_threshold: u64,
) -> Result<CohortComparison, ComparisonError> {
    validate(baseline)?;
    validate(candidate)?;
    if baseline.completed_observations < minimum_completed
        || candidate.completed_observations < minimum_completed
    {
        return Err(ComparisonError::InsufficientCompletedObservations);
    }
    let baseline_rate = rate(
        baseline.successful_observations,
        baseline.completed_observations,
    );
    let candidate_rate = rate(
        candidate.successful_observations,
        candidate.completed_observations,
    );
    let absolute_change = baseline_rate
        .zip(candidate_rate)
        .map(|(old, new)| new - old);
    let relative_change = baseline_rate
        .filter(|value| *value != 0.0)
        .zip(absolute_change)
        .map(|(old, delta)| delta / old);
    Ok(CohortComparison {
        baseline_rate,
        candidate_rate,
        absolute_change,
        relative_change,
        baseline_sample_size: baseline.sample_size,
        candidate_sample_size: candidate.sample_size,
        baseline_missing_data_rate: missing_rate(baseline),
        candidate_missing_data_rate: missing_rate(candidate),
        small_sample_warning: baseline.completed_observations < small_sample_threshold
            || candidate.completed_observations < small_sample_threshold,
        metric_definition: baseline.metric_definition.to_owned(),
    })
}

fn validate(input: CohortInput) -> Result<(), ComparisonError> {
    if input.successful_observations > input.completed_observations
        || input.completed_observations > input.sample_size
        || input.missing_observations > input.sample_size
    {
        Err(ComparisonError::InvalidCounts)
    } else {
        Ok(())
    }
}

fn rate(successes: u64, completed: u64) -> Option<f64> {
    (completed > 0).then(|| successes as f64 / completed as f64)
}

fn missing_rate(input: CohortInput) -> f64 {
    (input.sample_size > 0)
        .then(|| input.missing_observations as f64 / input.sample_size as f64)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cohort(sample_size: u64, completed: u64, success: u64, missing: u64) -> CohortInput {
        CohortInput {
            sample_size,
            completed_observations: completed,
            successful_observations: success,
            missing_observations: missing,
            metric_definition: "landing-rate-v1",
        }
    }

    #[test]
    fn reports_deltas_and_small_sample_warning() {
        let result = compare_cohorts(cohort(100, 90, 45, 10), cohort(100, 95, 57, 5), 50, 100)
            .expect("valid cohorts");
        assert_eq!(result.baseline_rate, Some(0.5));
        assert_eq!(result.candidate_rate, Some(0.6));
        assert!(
            (result.absolute_change.expect("delta") - 0.1).abs() < 1e-12,
            "unexpected delta: {:?}",
            result.absolute_change
        );
        assert!(result.small_sample_warning);
        assert_eq!(result.metric_definition, "landing-rate-v1");
    }

    #[test]
    fn rejects_incomplete_window() {
        assert_eq!(
            compare_cohorts(cohort(10, 2, 1, 8), cohort(10, 10, 5, 0), 3, 3),
            Err(ComparisonError::InsufficientCompletedObservations)
        );
    }
}
