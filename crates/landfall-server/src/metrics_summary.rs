use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct MetricSummary {
    pub numerator: u64,
    pub denominator: u64,
    pub excluded_in_flight: u64,
    pub excluded_unknown: u64,
    pub definition_version: String,
}

#[derive(Debug, Clone, Copy)]
pub struct MetricRow {
    pub terminal_eligible: bool,
    pub success: bool,
    pub in_flight: bool,
    pub unknown: bool,
}

pub fn summarize_metric(
    rows: impl IntoIterator<Item = MetricRow>,
    definition_version: impl Into<String>,
) -> MetricSummary {
    let mut summary = MetricSummary {
        numerator: 0,
        denominator: 0,
        excluded_in_flight: 0,
        excluded_unknown: 0,
        definition_version: definition_version.into(),
    };
    for row in rows {
        if row.in_flight {
            summary.excluded_in_flight += 1;
            continue;
        }
        if row.unknown {
            summary.excluded_unknown += 1;
            continue;
        }
        if row.terminal_eligible {
            summary.denominator += 1;
            if row.success {
                summary.numerator += 1;
            }
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn denominator_excludes_in_flight_and_unknown_rows() {
        let summary = summarize_metric(
            [
                MetricRow {
                    terminal_eligible: true,
                    success: true,
                    in_flight: false,
                    unknown: false,
                },
                MetricRow {
                    terminal_eligible: true,
                    success: false,
                    in_flight: false,
                    unknown: false,
                },
                MetricRow {
                    terminal_eligible: false,
                    success: false,
                    in_flight: true,
                    unknown: false,
                },
                MetricRow {
                    terminal_eligible: true,
                    success: false,
                    in_flight: false,
                    unknown: true,
                },
            ],
            "metric-v1",
        );
        assert_eq!(summary.numerator, 1);
        assert_eq!(summary.denominator, 2);
        assert_eq!(summary.excluded_in_flight, 1);
        assert_eq!(summary.excluded_unknown, 1);
    }
}
