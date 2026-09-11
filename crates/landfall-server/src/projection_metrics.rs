//! In-process projection and dead-job metrics.

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct ProjectionMetrics {
    projected_total: AtomicU64,
    failed_total: AtomicU64,
    dead_total: AtomicU64,
    lag_millis: AtomicU64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ProjectionMetricsSnapshot {
    pub projected_total: u64,
    pub failed_total: u64,
    pub dead_total: u64,
    pub lag_millis: u64,
}

impl ProjectionMetrics {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn record_projected(&self) {
        self.projected_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_failed(&self) {
        self.failed_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_dead(&self) {
        self.dead_total.fetch_add(1, Ordering::Relaxed);
    }
    pub fn set_lag_millis(&self, value: u64) {
        self.lag_millis.store(value, Ordering::Relaxed);
    }
    #[must_use]
    pub fn snapshot(&self) -> ProjectionMetricsSnapshot {
        ProjectionMetricsSnapshot {
            projected_total: self.projected_total.load(Ordering::Relaxed),
            failed_total: self.failed_total.load(Ordering::Relaxed),
            dead_total: self.dead_total.load(Ordering::Relaxed),
            lag_millis: self.lag_millis.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectionMetrics;
    #[test]
    fn snapshot_tracks_projection_and_dead_job_signals() {
        let metrics = ProjectionMetrics::new();
        metrics.record_projected();
        metrics.record_failed();
        metrics.record_dead();
        metrics.set_lag_millis(42);
        assert_eq!(metrics.snapshot().projected_total, 1);
        assert_eq!(metrics.snapshot().dead_total, 1);
        assert_eq!(metrics.snapshot().lag_millis, 42);
    }
}
