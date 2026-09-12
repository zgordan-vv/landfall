//! Projection throughput and watermark-lag measurement helpers.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectionBenchmarkResult {
    pub events: u64,
    pub elapsed_ms: u64,
    pub events_per_second: f64,
    pub watermark_lag: u64,
}

/// Normalizes projection work and current-vs-target watermark into benchmark metrics.
pub fn measure_projection(
    events: u64,
    elapsed_ms: u64,
    target_watermark: u64,
    current_watermark: u64,
) -> Option<ProjectionBenchmarkResult> {
    (events > 0 && elapsed_ms > 0).then(|| ProjectionBenchmarkResult {
        events,
        elapsed_ms,
        events_per_second: events as f64 / (elapsed_ms as f64 / 1_000.0),
        watermark_lag: target_watermark.saturating_sub(current_watermark),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_throughput_and_non_negative_lag() {
        let result = measure_projection(1_000, 2_000, 120, 100).unwrap();
        assert_eq!(result.events_per_second, 500.0);
        assert_eq!(result.watermark_lag, 20);
        assert_eq!(measure_projection(1, 0, 1, 0), None);
        assert_eq!(measure_projection(1, 1, 1, 2).unwrap().watermark_lag, 0);
    }
}
