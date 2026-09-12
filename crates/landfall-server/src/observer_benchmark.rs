//! Observer batch/RPC rate measurement helpers.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObserverRateResult {
    pub batches: u64,
    pub rpc_calls: u64,
    pub elapsed_ms: u64,
    pub calls_per_second: f64,
    pub calls_per_batch: f64,
    pub rate_limited_ratio: f64,
}

/// Normalizes observer counters captured during a benchmark window.
pub fn measure_observer(
    batches: u64,
    rpc_calls: u64,
    rate_limited: u64,
    elapsed_ms: u64,
) -> Option<ObserverRateResult> {
    (batches > 0 && rpc_calls > 0 && elapsed_ms > 0 && rate_limited <= rpc_calls).then(|| {
        ObserverRateResult {
            batches,
            rpc_calls,
            elapsed_ms,
            calls_per_second: rpc_calls as f64 / (elapsed_ms as f64 / 1_000.0),
            calls_per_batch: rpc_calls as f64 / batches as f64,
            rate_limited_ratio: rate_limited as f64 / rpc_calls as f64,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_batch_efficiency_and_rate_limits() {
        let result = measure_observer(10, 100, 5, 2_000).unwrap();
        assert_eq!(result.calls_per_second, 50.0);
        assert_eq!(result.calls_per_batch, 10.0);
        assert_eq!(result.rate_limited_ratio, 0.05);
        assert!(measure_observer(0, 1, 0, 1).is_none());
        assert!(measure_observer(1, 1, 2, 1).is_none());
    }
}
