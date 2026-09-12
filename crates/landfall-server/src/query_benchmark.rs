//! Query latency percentile helpers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryLatencyResult {
    pub samples: usize,
    pub p50_us: u64,
    pub p95_us: u64,
    pub max_us: u64,
}

/// Computes stable latency percentiles from collected microsecond samples.
pub fn summarize_query_latency(mut samples: Vec<u64>) -> Option<QueryLatencyResult> {
    if samples.is_empty() {
        return None;
    }
    samples.sort_unstable();
    let percentile = |percent: usize| samples[((samples.len() - 1) * percent) / 100];
    Some(QueryLatencyResult {
        samples: samples.len(),
        p50_us: percentile(50),
        p95_us: percentile(95),
        max_us: samples.last().copied()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn computes_trace_detail_percentiles_deterministically() {
        let result = summarize_query_latency((1..=100).collect()).unwrap();
        assert_eq!(result.samples, 100);
        assert_eq!(result.p50_us, 50);
        assert_eq!(result.p95_us, 95);
        assert_eq!(result.max_us, 100);
        assert!(summarize_query_latency(Vec::new()).is_none());
    }
}
