//! Database storage-size measurement helpers.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StorageSizeResult {
    pub raw_bytes: u64,
    pub events: u64,
    pub traces: u64,
    pub bytes_per_event: f64,
    pub bytes_per_trace: f64,
}

/// Calculates normalized storage costs from a measured fixture/database size.
pub fn storage_size(raw_bytes: u64, events: u64, traces: u64) -> Option<StorageSizeResult> {
    (events > 0 && traces > 0).then(|| StorageSizeResult {
        raw_bytes,
        events,
        traces,
        bytes_per_event: raw_bytes as f64 / events as f64,
        bytes_per_trace: raw_bytes as f64 / traces as f64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizes_measured_storage_and_rejects_zero_denominators() {
        let result = storage_size(10_000, 100, 10).unwrap();
        assert_eq!(result.bytes_per_event, 100.0);
        assert_eq!(result.bytes_per_trace, 1_000.0);
        assert!(storage_size(1, 0, 1).is_none());
        assert!(storage_size(1, 1, 0).is_none());
    }
}
