//! Report generation duration normalization for large cohorts.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReportDurationResult {
    pub traces: u64,
    pub elapsed_ms: u64,
    pub traces_per_second: f64,
}

/// Converts a measured report duration into a comparable throughput metric.
pub fn measure_report_duration(traces: u64, elapsed_ms: u64) -> Option<ReportDurationResult> {
    (traces > 0 && elapsed_ms > 0).then(|| ReportDurationResult {
        traces,
        elapsed_ms,
        traces_per_second: traces as f64 / (elapsed_ms as f64 / 1_000.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_large_report_duration() {
        let result = measure_report_duration(100_000, 10_000).unwrap();
        assert_eq!(result.traces_per_second, 10_000.0);
        assert!(measure_report_duration(0, 1).is_none());
    }
}
