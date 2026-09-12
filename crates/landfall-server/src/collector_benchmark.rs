//! In-process collector throughput benchmark helpers.

use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CollectorBenchmarkResult {
    pub events: usize,
    pub elapsed_ms: f64,
    pub events_per_second: f64,
    pub p95_latency_us: u64,
}

/// Measures deterministic JSON parse work and reports throughput/p95 latency.
pub fn measure_collector(
    payload: &[u8],
    events: usize,
) -> Result<CollectorBenchmarkResult, serde_json::Error> {
    let mut samples = Vec::with_capacity(events);
    let started = Instant::now();
    for _ in 0..events {
        let sample_started = Instant::now();
        let _: serde_json::Value = serde_json::from_slice(payload)?;
        samples.push(sample_started.elapsed().as_micros() as u64);
    }
    samples.sort_unstable();
    let elapsed = started.elapsed().as_secs_f64();
    let p95_index =
        ((samples.len().saturating_sub(1) * 95) / 100).min(samples.len().saturating_sub(1));
    Ok(CollectorBenchmarkResult {
        events,
        elapsed_ms: elapsed * 1000.0,
        events_per_second: events as f64 / elapsed.max(f64::EPSILON),
        p95_latency_us: samples.get(p95_index).copied().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_throughput_and_rejects_malformed_payload() {
        let result = measure_collector(br#"{"event_type":"fixture"}"#, 100).unwrap();
        assert_eq!(result.events, 100);
        assert!(result.events_per_second > 0.0);
        assert!(measure_collector(b"not-json", 1).is_err());
    }
}
