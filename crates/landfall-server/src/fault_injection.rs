//! Deterministic fault-injection scenarios for resilience checks.

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FailureKind {
    CollectorDown,
    DatabaseDown,
    ProjectorCrash,
    ObserverRateLimited,
    CorruptRpcResponse,
    DiskPressure,
    ReportWorkerCrash,
    RetentionFailure,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SafeOutcome {
    Retryable,
    HaltWrites,
    Rehydrate,
    Quarantine,
    DryRunOnly,
}

/// Maps an injected failure to the required non-destructive response.
#[must_use]
pub const fn expected_outcome(failure: FailureKind) -> SafeOutcome {
    match failure {
        FailureKind::CollectorDown | FailureKind::ObserverRateLimited => SafeOutcome::Retryable,
        FailureKind::DatabaseDown | FailureKind::DiskPressure => SafeOutcome::HaltWrites,
        FailureKind::ProjectorCrash | FailureKind::ReportWorkerCrash => SafeOutcome::Rehydrate,
        FailureKind::CorruptRpcResponse => SafeOutcome::Quarantine,
        FailureKind::RetentionFailure => SafeOutcome::DryRunOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_failure_has_explicit_safe_outcome() {
        assert_eq!(
            expected_outcome(FailureKind::CollectorDown),
            SafeOutcome::Retryable
        );
        assert_eq!(
            expected_outcome(FailureKind::DatabaseDown),
            SafeOutcome::HaltWrites
        );
        assert_eq!(
            expected_outcome(FailureKind::ProjectorCrash),
            SafeOutcome::Rehydrate
        );
        assert_eq!(
            expected_outcome(FailureKind::ObserverRateLimited),
            SafeOutcome::Retryable
        );
        assert_eq!(
            expected_outcome(FailureKind::CorruptRpcResponse),
            SafeOutcome::Quarantine
        );
        assert_eq!(
            expected_outcome(FailureKind::DiskPressure),
            SafeOutcome::HaltWrites
        );
        assert_eq!(
            expected_outcome(FailureKind::ReportWorkerCrash),
            SafeOutcome::Rehydrate
        );
        assert_eq!(
            expected_outcome(FailureKind::RetentionFailure),
            SafeOutcome::DryRunOnly
        );
    }
}
