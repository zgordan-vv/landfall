//! Retention partition-drop duration normalization.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetentionDurationResult {
    pub partitions: u64,
    pub elapsed_ms: u64,
    pub partitions_per_second: f64,
    pub dry_run: bool,
}

/// Converts a measured partition operation into a comparable rate.
pub fn measure_retention(
    partitions: u64,
    elapsed_ms: u64,
    dry_run: bool,
) -> Option<RetentionDurationResult> {
    (partitions > 0 && elapsed_ms > 0).then(|| RetentionDurationResult {
        partitions,
        elapsed_ms,
        partitions_per_second: partitions as f64 / (elapsed_ms as f64 / 1_000.0),
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_partition_rate_and_mode() {
        let result = measure_retention(20, 4_000, true).unwrap();
        assert_eq!(result.partitions_per_second, 5.0);
        assert!(result.dry_run);
        assert!(measure_retention(0, 1, false).is_none());
    }
}
