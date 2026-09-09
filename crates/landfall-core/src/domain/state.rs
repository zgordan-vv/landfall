//! Orthogonal transaction-state dimensions.
//!
//! These types describe a projection result. They deliberately do not encode
//! event transitions; canonical ordering and reduction own that behavior.

use landfall_protocol::{BusinessOutcome, Commitment, ExecutionResult};

/// Furthest technical lifecycle stage supported by immutable evidence.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LifecycleStage {
    /// A trace exists, including optional construction or simulation evidence.
    #[default]
    Created,
    /// Signed transaction evidence exists.
    Signed,
    /// At least one real submission invocation occurred.
    Submitted,
    /// At least one network status observation occurred, including not-found.
    Observed,
}

/// Best defensible statement about network inclusion and commitment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum LandingState {
    /// No configured observer has reported inclusion.
    #[default]
    NotObserved,
    /// Inclusion was observed at processed commitment.
    Processed,
    /// Inclusion was observed at confirmed commitment.
    Confirmed,
    /// Inclusion was observed at finalized commitment.
    Finalized,
    /// The validity window passed after sufficient no-inclusion observation.
    Expired,
    /// Observation stopped without enough evidence for a terminal conclusion.
    Incomplete,
    /// Retained observers or later facts disagree about inclusion.
    Conflicting,
}

impl LandingState {
    /// Returns the observed commitment when this is an unambiguous inclusion.
    #[must_use]
    pub const fn commitment(self) -> Option<Commitment> {
        match self {
            Self::Processed => Some(Commitment::Processed),
            Self::Confirmed => Some(Commitment::Confirmed),
            Self::Finalized => Some(Commitment::Finalized),
            Self::NotObserved | Self::Expired | Self::Incomplete | Self::Conflicting => None,
        }
    }

    /// Whether unambiguous network inclusion was observed.
    #[must_use]
    pub const fn is_landed(self) -> bool {
        matches!(self, Self::Processed | Self::Confirmed | Self::Finalized)
    }
}

/// On-chain instruction execution result, independent of landing commitment.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ExecutionState {
    /// No defensible execution result is available.
    #[default]
    Unknown,
    /// Included transaction executed without an on-chain error.
    Success,
    /// Included transaction produced an on-chain execution error.
    Failure,
}

impl From<ExecutionResult> for ExecutionState {
    fn from(value: ExecutionResult) -> Self {
        match value {
            ExecutionResult::Success => Self::Success,
            ExecutionResult::Failure => Self::Failure,
        }
    }
}

/// Customer application's reconciliation result, independent of the network.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ApplicationOutcome {
    /// The application supplied no conclusive business result.
    #[default]
    Unknown,
    /// The application reported its business operation as successful.
    Success,
    /// The application reported its business operation as failed.
    Failure,
    /// The application stopped waiting because of its own timeout boundary.
    Timeout,
    /// The application explicitly cancelled or abandoned the operation.
    Cancelled,
}

impl From<BusinessOutcome> for ApplicationOutcome {
    fn from(value: BusinessOutcome) -> Self {
        match value {
            BusinessOutcome::Success => Self::Success,
            BusinessOutcome::Failure => Self::Failure,
            BusinessOutcome::Timeout => Self::Timeout,
            BusinessOutcome::Cancelled => Self::Cancelled,
            BusinessOutcome::Unknown => Self::Unknown,
        }
    }
}

/// Whether the configured observation policy reached a defensible boundary.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ObservationCompleteness {
    /// Observers are still eligible to collect evidence.
    #[default]
    InProgress,
    /// The configured success or no-inclusion terminal policy was satisfied.
    Complete,
    /// Observation ended without evidence required by the policy.
    Incomplete,
}

impl ObservationCompleteness {
    /// Whether observation has stopped for terminal-metric purposes.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Complete | Self::Incomplete)
    }

    /// Whether this trace may enter a terminal-rate denominator.
    #[must_use]
    pub const fn is_metric_eligible(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Complete current state without collapsing independent claims into one enum.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TraceState {
    /// Furthest observed technical stage.
    pub lifecycle: LifecycleStage,
    /// Network inclusion and commitment statement.
    pub landing: LandingState,
    /// On-chain instruction outcome.
    pub execution: ExecutionState,
    /// Customer application's business reconciliation result.
    pub application: ApplicationOutcome,
    /// Observation-window completeness.
    pub observation: ObservationCompleteness,
}

impl TraceState {
    /// Validates only cross-dimension facts that are always contradictory.
    pub const fn validate(self) -> Result<(), StateInvariantError> {
        if self.landing.is_landed() && !matches!(self.lifecycle, LifecycleStage::Observed) {
            return Err(StateInvariantError::LandingWithoutObservation);
        }
        if !matches!(self.execution, ExecutionState::Unknown)
            && !self.landing.is_landed()
            && !matches!(self.landing, LandingState::Conflicting)
        {
            return Err(StateInvariantError::ExecutionWithoutInclusion);
        }
        if matches!(self.landing, LandingState::Expired)
            && !matches!(self.observation, ObservationCompleteness::Complete)
        {
            return Err(StateInvariantError::ExpirationWithoutCompleteObservation);
        }
        if matches!(self.landing, LandingState::Incomplete)
            && !matches!(self.observation, ObservationCompleteness::Incomplete)
        {
            return Err(StateInvariantError::IncompleteLandingWithoutIncompleteObservation);
        }
        Ok(())
    }
}

/// An impossible relationship between otherwise independent state dimensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StateInvariantError {
    /// Inclusion commitment requires at least one network observation.
    LandingWithoutObservation,
    /// A known execution result requires inclusion evidence or a conflict.
    ExecutionWithoutInclusion,
    /// Expiration requires a completed observation window.
    ExpirationWithoutCompleteObservation,
    /// The landing marker and observation-completeness marker disagree.
    IncompleteLandingWithoutIncompleteObservation,
}

impl std::fmt::Display for StateInvariantError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LandingWithoutObservation => {
                formatter.write_str("landing evidence requires the observed lifecycle stage")
            }
            Self::ExecutionWithoutInclusion => {
                formatter.write_str("known execution requires inclusion evidence")
            }
            Self::ExpirationWithoutCompleteObservation => {
                formatter.write_str("expiration requires a completed observation window")
            }
            Self::IncompleteLandingWithoutIncompleteObservation => formatter
                .write_str("incomplete landing requires incomplete observation completeness"),
        }
    }
}

impl std::error::Error for StateInvariantError {}
