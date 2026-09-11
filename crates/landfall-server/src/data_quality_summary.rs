//! Stable read model for aggregate data-quality coverage and gaps.

use std::collections::BTreeMap;

use landfall_core::data_quality::{
    DATA_QUALITY_VERSION, DataQualityAssessment, DataQualityFindingCode, DataQualityGrade,
};
use serde::Serialize;
use utoipa::ToSchema;

/// Aggregate quality distribution for a selected set of reduced traces.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct DataQualitySummary {
    /// Number of assessments included in the summary.
    pub assessments: u64,
    /// Number of traces in each quality grade, in stable A-to-F order.
    pub grades: Vec<GradeCount>,
    /// Number of traces reporting each distinct quality gap.
    pub gaps: Vec<GapCount>,
    /// Average score, rounded down; `None` when no assessments were supplied.
    pub average_score: Option<u8>,
    /// Rubric version used by the source assessments.
    pub definition_version: String,
}

/// Count for one [`DataQualityGrade`].
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct GradeCount {
    pub grade: char,
    pub count: u64,
}

/// Count for one stable, machine-readable quality-gap key.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct GapCount {
    pub gap: String,
    pub count: u64,
}

/// Builds a deterministic summary without turning absent evidence into success.
#[must_use]
pub fn summarize_data_quality(
    assessments: impl IntoIterator<Item = DataQualityAssessment>,
) -> DataQualitySummary {
    let mut grade_counts = [0_u64; 5];
    let mut gap_counts = BTreeMap::<String, u64>::new();
    let mut total_score = 0_u64;
    let mut total = 0_u64;

    for assessment in assessments {
        total += 1;
        total_score += u64::from(assessment.score());
        let grade_index = match assessment.grade() {
            DataQualityGrade::A => 0,
            DataQualityGrade::B => 1,
            DataQualityGrade::C => 2,
            DataQualityGrade::D => 3,
            DataQualityGrade::F => 4,
        };
        grade_counts[grade_index] += 1;
        for finding in assessment.findings() {
            *gap_counts.entry(finding_key(finding.code())).or_default() += 1;
        }
    }

    let grades = ['A', 'B', 'C', 'D', 'F']
        .into_iter()
        .zip(grade_counts)
        .map(|(grade, count)| GradeCount { grade, count })
        .collect();
    let gaps = gap_counts
        .into_iter()
        .map(|(gap, count)| GapCount { gap, count })
        .collect();

    DataQualitySummary {
        assessments: total,
        grades,
        gaps,
        average_score: (total > 0).then(|| (total_score / total) as u8),
        definition_version: DATA_QUALITY_VERSION.to_owned(),
    }
}

fn finding_key(code: DataQualityFindingCode) -> String {
    match code {
        DataQualityFindingCode::Reported(category) => {
            let name = serde_json::to_value(category)
                .ok()
                .and_then(|value| value.as_str().map(str::to_owned))
                .unwrap_or_else(|| "unknown".to_owned());
            format!("reported:{name}")
        }
        DataQualityFindingCode::MissingSubmissionResponse { .. } => {
            "missing_submission_response".to_owned()
        }
        DataQualityFindingCode::MissingRetrySource { .. } => "missing_retry_source".to_owned(),
        other => format!("{:?}", other).to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_summary_is_explicit() {
        let summary = summarize_data_quality([]);
        assert_eq!(summary.assessments, 0);
        assert_eq!(summary.average_score, None);
        assert_eq!(summary.grades.len(), 5);
        assert!(summary.gaps.is_empty());
    }
}
