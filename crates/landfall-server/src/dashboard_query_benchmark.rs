//! Overview/comparison dashboard query latency summaries.

use super::query_benchmark::{QueryLatencyResult, summarize_query_latency};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardQueryResult {
    pub overview: Option<QueryLatencyResult>,
    pub comparison: Option<QueryLatencyResult>,
}

/// Summarizes dashboard query classes independently to avoid hiding a slow path.
#[must_use]
pub fn summarize_dashboard_queries(
    overview_samples: Vec<u64>,
    comparison_samples: Vec<u64>,
) -> DashboardQueryResult {
    DashboardQueryResult {
        overview: summarize_query_latency(overview_samples),
        comparison: summarize_query_latency(comparison_samples),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_overview_and_comparison_percentiles_separate() {
        let result = summarize_dashboard_queries((1..=20).collect(), (100..=119).collect());
        assert_eq!(result.overview.unwrap().p95_us, 19);
        assert_eq!(result.comparison.unwrap().p95_us, 118);
        assert!(
            summarize_dashboard_queries(Vec::new(), vec![1])
                .overview
                .is_none()
        );
    }
}
