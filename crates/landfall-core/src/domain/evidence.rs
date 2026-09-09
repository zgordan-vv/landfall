//! Evidence references shared by domain entities.

use std::collections::{BTreeSet, btree_set};

use landfall_protocol::EventId;

use super::DomainInvariantError;

/// Non-empty deduplicated set of immutable source-event identities.
///
/// The UUID order is stable for equality and output stability, but is not the
/// semantic event order used by the reducer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSet(BTreeSet<EventId>);

impl EvidenceSet {
    /// Starts an evidence set with its required first event.
    #[must_use]
    pub fn new(first: EventId) -> Self {
        Self(BTreeSet::from([first]))
    }

    /// Builds a deduplicated set and rejects an empty input.
    pub fn try_from_events(
        events: impl IntoIterator<Item = EventId>,
        entity: &'static str,
    ) -> Result<Self, DomainInvariantError> {
        let events = events.into_iter().collect::<BTreeSet<_>>();
        if events.is_empty() {
            Err(DomainInvariantError::EmptyEvidence { entity })
        } else {
            Ok(Self(events))
        }
    }

    /// Adds evidence, returning whether the event was not already present.
    pub fn insert(&mut self, event_id: EventId) -> bool {
        self.0.insert(event_id)
    }

    /// Returns whether this set contains an event.
    #[must_use]
    pub fn contains(&self, event_id: &EventId) -> bool {
        self.0.contains(event_id)
    }

    /// Returns the number of unique source events.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Evidence sets are never empty after construction.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Iterates source IDs in stable UUID order, not semantic event order.
    pub fn iter(&self) -> btree_set::Iter<'_, EventId> {
        self.0.iter()
    }
}

impl<'a> IntoIterator for &'a EvidenceSet {
    type Item = &'a EventId;
    type IntoIter = btree_set::Iter<'a, EventId>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Optional start/completion evidence for an operation observed from either side.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LifecycleEvidence {
    started_event_id: Option<EventId>,
    completed_event_id: Option<EventId>,
}

impl LifecycleEvidence {
    /// Creates lifecycle evidence while preserving missing starts/completions.
    pub fn new(
        started_event_id: Option<EventId>,
        completed_event_id: Option<EventId>,
    ) -> Result<Self, DomainInvariantError> {
        if started_event_id.is_none() && completed_event_id.is_none() {
            return Err(DomainInvariantError::EmptyEvidence {
                entity: "lifecycle operation",
            });
        }
        if started_event_id.is_some() && started_event_id == completed_event_id {
            return Err(DomainInvariantError::ReusedLifecycleEvent);
        }
        Ok(Self {
            started_event_id,
            completed_event_id,
        })
    }

    /// Event that observed the operation start, when captured.
    #[must_use]
    pub const fn started_event_id(self) -> Option<EventId> {
        self.started_event_id
    }

    /// Event that observed the operation completion, when captured.
    #[must_use]
    pub const fn completed_event_id(self) -> Option<EventId> {
        self.completed_event_id
    }
}
