//! Deterministic synthetic workloads for capacity benchmarks.

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SyntheticOutcome {
    Success,
    Expired,
    ExecutionError,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SyntheticTrace {
    pub trace_index: u64,
    pub event_count: u8,
    pub outcome: SyntheticOutcome,
    pub route_index: u16,
    pub app_version: u8,
}

/// Generates repeatable traces from a seed without UUID/time randomness.
#[must_use]
pub fn generate_traces(count: usize, seed: u64) -> Vec<SyntheticTrace> {
    (0..count)
        .map(|index| {
            let value = seed.wrapping_add(index as u64);
            SyntheticTrace {
                trace_index: index as u64,
                event_count: 8 + (value % 9) as u8,
                outcome: match value % 10 {
                    0..=6 => SyntheticOutcome::Success,
                    7..=8 => SyntheticOutcome::Expired,
                    _ => SyntheticOutcome::ExecutionError,
                },
                route_index: (value % 4) as u16,
                app_version: (value % 3) as u8,
            }
        })
        .collect()
}

/// Number of events emitted by a one-second burst workload.
#[must_use]
pub const fn burst_event_count(events_per_second: u32) -> u32 {
    events_per_second
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generator_is_repeatable_and_covers_mixed_dimensions() {
        let first = generate_traces(1_000, 7);
        assert_eq!(first, generate_traces(1_000, 7));
        assert_eq!(first.len(), 1_000);
        assert!(
            first
                .iter()
                .any(|trace| trace.outcome == SyntheticOutcome::Expired)
        );
        assert!(
            first
                .iter()
                .any(|trace| trace.outcome == SyntheticOutcome::ExecutionError)
        );
        assert!(first.iter().any(|trace| trace.route_index > 0));
        assert_eq!(burst_event_count(500), 500);
    }
}
