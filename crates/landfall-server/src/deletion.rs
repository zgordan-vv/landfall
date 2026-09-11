//! Trace deletion jobs and append-only tombstone/audit semantics.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TraceTombstone {
    pub trace_id: String,
    pub deleted_at: String,
    pub actor: String,
    pub reason: String,
}

/// Durable work item consumed by the deletion worker.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DeletionJob {
    pub trace_id: String,
    pub tombstone: TraceTombstone,
}

/// Applies one deletion job to the append-only ledger.
pub fn execute_deletion_job(
    ledger: &mut DeletionLedger,
    job: DeletionJob,
) -> Result<(), DeletionError> {
    ledger.delete(job.tombstone)
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DeletionError {
    EmptyTrace,
    EmptyActor,
    EmptyTimestamp,
    AlreadyDeleted,
}

#[derive(Debug, Default)]
pub struct DeletionLedger {
    tombstones: BTreeMap<String, TraceTombstone>,
}

impl DeletionLedger {
    /// Records a tombstone once; later requests cannot resurrect the trace.
    pub fn delete(&mut self, tombstone: TraceTombstone) -> Result<(), DeletionError> {
        if tombstone.trace_id.trim().is_empty() {
            return Err(DeletionError::EmptyTrace);
        }
        if tombstone.actor.trim().is_empty() {
            return Err(DeletionError::EmptyActor);
        }
        if tombstone.deleted_at.trim().is_empty() {
            return Err(DeletionError::EmptyTimestamp);
        }
        if self.tombstones.contains_key(&tombstone.trace_id) {
            return Err(DeletionError::AlreadyDeleted);
        }
        self.tombstones
            .insert(tombstone.trace_id.clone(), tombstone);
        Ok(())
    }
    #[must_use]
    pub fn is_deleted(&self, trace_id: &str) -> bool {
        self.tombstones.contains_key(trace_id)
    }
    #[must_use]
    pub fn audit(&self, trace_id: &str) -> Option<&TraceTombstone> {
        self.tombstones.get(trace_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tombstone_is_append_only_and_hides_trace() {
        let mut ledger = DeletionLedger::default();
        let tombstone = TraceTombstone {
            trace_id: "trace-1".into(),
            deleted_at: "2026-01-01T00:00:00Z".into(),
            actor: "operator".into(),
            reason: "customer request".into(),
        };
        ledger.delete(tombstone.clone()).unwrap();
        assert!(ledger.is_deleted("trace-1"));
        assert_eq!(ledger.audit("trace-1"), Some(&tombstone));
        assert_eq!(ledger.delete(tombstone), Err(DeletionError::AlreadyDeleted));
    }
}
