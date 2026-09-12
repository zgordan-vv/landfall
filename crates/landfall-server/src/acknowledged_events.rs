//! Verification that acknowledged ingestion events reached durable storage.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DurabilityGap {
    pub event_id: String,
}

/// Returns acknowledged IDs missing from the durable event set.
#[must_use]
pub fn find_durability_gaps(
    acknowledged: impl IntoIterator<Item = String>,
    durable: &BTreeSet<String>,
) -> Vec<DurabilityGap> {
    let mut gaps: Vec<_> = acknowledged
        .into_iter()
        .filter(|id| !durable.contains(id))
        .map(|event_id| DurabilityGap { event_id })
        .collect();
    gaps.sort_by(|left, right| left.event_id.cmp(&right.event_id));
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_missing_acknowledged_event_without_false_success() {
        let durable = BTreeSet::from(["e1".into(), "e3".into()]);
        assert!(find_durability_gaps(["e1".into(), "e3".into()], &durable).is_empty());
        assert_eq!(
            find_durability_gaps(["e2".into(), "e1".into()], &durable),
            vec![DurabilityGap {
                event_id: "e2".into()
            }]
        );
    }
}
