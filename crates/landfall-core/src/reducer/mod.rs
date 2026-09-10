//! Pure reconstruction of one trace projection from canonical immutable events.

use landfall_protocol::{
    BusinessActionId, BusinessOutcome, Commitment, DataQualityCategory, DataQualityImpact,
    EnvironmentId, EventId, ExecutionResult, ProjectId, SigningResult, StatusSourceResult, TraceId,
    WireEvent,
};

use crate::{
    domain::{
        ApplicationOutcome, EvidenceSet, ExecutionState, LandingState, LifecycleStage,
        ObservationCompleteness, StateInvariantError, TraceState, TransactionTrace,
    },
    ordering::CanonicalOrder,
};

/// Stable identity of the reducer semantics used to create a projection.
pub const REDUCER_VERSION: &str = "trace-reducer-v1";

/// Fresh authoritative projection derived from a complete canonical event set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceProjection {
    project_id: ProjectId,
    trace: TransactionTrace,
    warnings: Vec<ReducerWarning>,
}

impl TraceProjection {
    /// Owning project validated across every source event.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// Evidence-backed trace entity and its independently reduced state dimensions.
    #[must_use]
    pub const fn trace(&self) -> &TransactionTrace {
        &self.trace
    }

    /// Non-fatal ambiguities retained by reduction instead of silently discarded.
    #[must_use]
    pub fn warnings(&self) -> &[ReducerWarning] {
        &self.warnings
    }

    /// Reducer semantics that produced this projection.
    #[must_use]
    pub const fn reducer_version(&self) -> &'static str {
        REDUCER_VERSION
    }
}

/// Ambiguous evidence that still permits a defensible current projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReducerWarning {
    /// The trace was reconstructed from partial evidence without its creation event.
    MissingTraceCreated,
    /// More than one creation fact exists for the same immutable trace.
    MultipleTraceCreated {
        /// Number of distinct creation events.
        count: usize,
    },
    /// Both successful and failed on-chain executions were retained.
    ConflictingExecutionEvidence,
    /// More than one conclusive application outcome was retained.
    ConflictingApplicationOutcomes,
    /// Explicit data-quality evidence says configured observers disagreed.
    ObserverDisagreement,
}

/// Identity or invariant failure that prevents a trustworthy trace projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReducerError {
    /// A trace cannot be projected without evidence.
    EmptyInput,
    /// A project-scoped or action-only event was incorrectly supplied as trace evidence.
    MissingTraceId {
        /// Event outside a trace scope.
        event_id: EventId,
    },
    /// Events for different projects were mixed.
    ProjectMismatch {
        /// First event from the conflicting project.
        event_id: EventId,
    },
    /// Events for different environments were mixed.
    EnvironmentMismatch {
        /// First event from the conflicting environment.
        event_id: EventId,
    },
    /// Events for different traces were mixed.
    TraceMismatch {
        /// First event from the conflicting trace.
        event_id: EventId,
    },
    /// Two explicit business-action identities claim the same trace.
    BusinessActionMismatch {
        /// First event carrying the conflicting action identity.
        event_id: EventId,
    },
    /// Reduced dimensions violated a cross-dimension domain invariant.
    InvalidState(StateInvariantError),
}

impl std::fmt::Display for ReducerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("trace reduction requires at least one event"),
            Self::MissingTraceId { event_id } => {
                write!(formatter, "event {event_id} is not scoped to a trace")
            }
            Self::ProjectMismatch { event_id } => {
                write!(formatter, "event {event_id} belongs to another project")
            }
            Self::EnvironmentMismatch { event_id } => {
                write!(formatter, "event {event_id} belongs to another environment")
            }
            Self::TraceMismatch { event_id } => {
                write!(formatter, "event {event_id} belongs to another trace")
            }
            Self::BusinessActionMismatch { event_id } => write!(
                formatter,
                "event {event_id} assigns another business action to the trace"
            ),
            Self::InvalidState(error) => {
                write!(formatter, "reducer produced invalid state: {error}")
            }
        }
    }
}

impl std::error::Error for ReducerError {}

impl From<StateInvariantError> for ReducerError {
    fn from(value: StateInvariantError) -> Self {
        Self::InvalidState(value)
    }
}

/// Rebuilds one trace projection without storage, network, or prior projection state.
///
/// The input must already have passed [`crate::ordering::canonical_order`]. Every
/// invocation starts from defaults so replaying the same canonical evidence is
/// idempotent and cannot inherit stale derived fields.
pub fn reduce_trace(events: &CanonicalOrder) -> Result<TraceProjection, ReducerError> {
    let first = events.events().first().ok_or(ReducerError::EmptyInput)?;
    let base_identity = identity(&first.event);
    let trace_id = base_identity.trace.ok_or(ReducerError::MissingTraceId {
        event_id: base_identity.event,
    })?;
    let mut business_action_id = base_identity.business_action;
    let mut accumulator = Accumulator::default();
    let mut evidence_ids = Vec::with_capacity(events.events().len());

    for collected in events.events() {
        let current = identity(&collected.event);
        validate_identity(base_identity, trace_id, business_action_id, current)?;
        if business_action_id.is_none() {
            business_action_id = current.business_action;
        }
        evidence_ids.push(current.event);
        accumulator.apply(&collected.event);
    }

    let (state, warnings) = accumulator.finish()?;
    let evidence = EvidenceSet::try_from_events(evidence_ids, "trace")
        .map_err(|_| ReducerError::EmptyInput)?;
    let trace = TransactionTrace::with_state(
        trace_id,
        base_identity.environment,
        business_action_id,
        evidence,
        state,
    )?;

    Ok(TraceProjection {
        project_id: base_identity.project,
        trace,
        warnings,
    })
}

fn validate_identity(
    expected: EventIdentity,
    trace_id: TraceId,
    business_action_id: Option<BusinessActionId>,
    current: EventIdentity,
) -> Result<(), ReducerError> {
    if current.project != expected.project {
        return Err(ReducerError::ProjectMismatch {
            event_id: current.event,
        });
    }
    if current.environment != expected.environment {
        return Err(ReducerError::EnvironmentMismatch {
            event_id: current.event,
        });
    }
    match current.trace {
        None => {
            return Err(ReducerError::MissingTraceId {
                event_id: current.event,
            });
        }
        Some(current_trace_id) if current_trace_id != trace_id => {
            return Err(ReducerError::TraceMismatch {
                event_id: current.event,
            });
        }
        Some(_) => {}
    }
    if let (Some(expected_action), Some(current_action)) =
        (business_action_id, current.business_action)
        && current_action != expected_action
    {
        return Err(ReducerError::BusinessActionMismatch {
            event_id: current.event,
        });
    }
    Ok(())
}

const EXECUTION_SUCCESS: u8 = 1 << 0;
const EXECUTION_FAILURE: u8 = 1 << 1;
const DURABLE_NONCE: u8 = 1 << 0;
const CONFIRMATION_TARGET_REACHED: u8 = 1 << 1;
const OBSERVATION_INCOMPLETE: u8 = 1 << 2;
const OBSERVER_DISAGREEMENT: u8 = 1 << 3;

#[derive(Default)]
struct Accumulator {
    lifecycle: LifecycleStage,
    best_commitment: Option<Commitment>,
    execution: ExecutionState,
    execution_mask: u8,
    application: ApplicationOutcome,
    application_mask: u8,
    trace_created_count: usize,
    last_valid_block_height: Option<u64>,
    greatest_not_found_height: Option<u64>,
    evidence_flags: u8,
}

impl Accumulator {
    fn apply(&mut self, event: &WireEvent) {
        match event {
            WireEvent::TraceCreated(event) => {
                self.trace_created_count += 1;
                if event.attributes.uses_durable_nonce == Some(true) {
                    self.evidence_flags |= DURABLE_NONCE;
                }
            }
            WireEvent::BlockhashAcquired(event) => {
                if let Some(height) = event.attributes.last_valid_block_height {
                    self.last_valid_block_height = Some(
                        self.last_valid_block_height
                            .map_or(height.get(), |current| current.max(height.get())),
                    );
                }
            }
            WireEvent::SigningCompleted(event)
                if event.attributes.result == SigningResult::Completed =>
            {
                self.advance(LifecycleStage::Signed);
            }
            WireEvent::SubmissionStarted(_)
            | WireEvent::SubmissionCompleted(_)
            | WireEvent::SubmissionRetryScheduled(_)
            | WireEvent::ConfirmationWaitStarted(_) => {
                self.advance(LifecycleStage::Submitted);
            }
            WireEvent::ConfirmationWaitCompleted(event) => {
                self.advance(LifecycleStage::Submitted);
                if let Some(commitment) = event.attributes.observed_commitment {
                    self.advance(LifecycleStage::Observed);
                    self.observe_commitment(commitment);
                }
                if matches!(
                    event.attributes.result,
                    landfall_protocol::ConfirmationWaitResult::CommitmentReached
                ) {
                    self.evidence_flags |= CONFIRMATION_TARGET_REACHED;
                }
            }
            WireEvent::StatusObserved(event) => {
                self.advance(LifecycleStage::Observed);
                if event.attributes.source_result == StatusSourceResult::Found {
                    if let Some(commitment) = event.attributes.commitment {
                        self.observe_commitment(commitment);
                    }
                } else if event.attributes.source_result == StatusSourceResult::NotFound
                    && let Some(height) = event.attributes.block_height
                {
                    self.greatest_not_found_height = Some(
                        self.greatest_not_found_height
                            .map_or(height.get(), |current| current.max(height.get())),
                    );
                }
            }
            WireEvent::ExecutionEnriched(event) => {
                self.advance(LifecycleStage::Observed);
                self.observe_commitment(event.attributes.commitment);
                self.execution = ExecutionState::from(event.attributes.execution_result);
                match event.attributes.execution_result {
                    ExecutionResult::Success => self.execution_mask |= EXECUTION_SUCCESS,
                    ExecutionResult::Failure => self.execution_mask |= EXECUTION_FAILURE,
                }
            }
            WireEvent::BusinessOutcomeObserved(event) => {
                self.application = ApplicationOutcome::from(event.attributes.outcome);
                self.application_mask |= application_bit(event.attributes.outcome);
            }
            WireEvent::DataQualityDetected(event) => {
                if event.attributes.impact == DataQualityImpact::ObservationIncomplete {
                    self.evidence_flags |= OBSERVATION_INCOMPLETE;
                }
                if event.attributes.category == DataQualityCategory::ObserverDisagreement {
                    self.evidence_flags |= OBSERVER_DISAGREEMENT;
                }
            }
            WireEvent::SimulationStarted(_)
            | WireEvent::SimulationCompleted(_)
            | WireEvent::SigningStarted(_)
            | WireEvent::SigningCompleted(_) => {}
        }
    }

    fn advance(&mut self, stage: LifecycleStage) {
        self.lifecycle = self.lifecycle.max(stage);
    }

    fn observe_commitment(&mut self, candidate: Commitment) {
        if self
            .best_commitment
            .is_none_or(|current| commitment_rank(candidate) > commitment_rank(current))
        {
            self.best_commitment = Some(candidate);
        }
    }

    fn finish(self) -> Result<(TraceState, Vec<ReducerWarning>), ReducerError> {
        let expired = self.evidence_flags & DURABLE_NONCE == 0
            && self
                .last_valid_block_height
                .zip(self.greatest_not_found_height)
                .is_some_and(|(valid, observed)| observed > valid);
        let landing =
            if self.evidence_flags & OBSERVER_DISAGREEMENT != 0 && self.best_commitment.is_some() {
                LandingState::Conflicting
            } else if let Some(commitment) = self.best_commitment {
                landing_from(commitment)
            } else if expired {
                LandingState::Expired
            } else if self.evidence_flags & OBSERVATION_INCOMPLETE != 0 {
                LandingState::Incomplete
            } else {
                LandingState::NotObserved
            };
        let observation = if self.evidence_flags & CONFIRMATION_TARGET_REACHED != 0
            || self.best_commitment == Some(Commitment::Finalized)
            || expired
        {
            ObservationCompleteness::Complete
        } else if self.evidence_flags & OBSERVATION_INCOMPLETE != 0 {
            ObservationCompleteness::Incomplete
        } else {
            ObservationCompleteness::InProgress
        };
        let state = TraceState {
            lifecycle: self.lifecycle,
            landing,
            execution: self.execution,
            application: self.application,
            observation,
        };
        state.validate()?;

        let mut warnings = Vec::new();
        match self.trace_created_count {
            0 => warnings.push(ReducerWarning::MissingTraceCreated),
            1 => {}
            count => warnings.push(ReducerWarning::MultipleTraceCreated { count }),
        }
        if self.execution_mask == EXECUTION_SUCCESS | EXECUTION_FAILURE {
            warnings.push(ReducerWarning::ConflictingExecutionEvidence);
        }
        if self.application_mask.count_ones() > 1 {
            warnings.push(ReducerWarning::ConflictingApplicationOutcomes);
        }
        if self.evidence_flags & OBSERVER_DISAGREEMENT != 0 {
            warnings.push(ReducerWarning::ObserverDisagreement);
        }
        Ok((state, warnings))
    }
}

const fn commitment_rank(commitment: Commitment) -> u8 {
    match commitment {
        Commitment::Processed => 1,
        Commitment::Confirmed => 2,
        Commitment::Finalized => 3,
    }
}

const fn landing_from(commitment: Commitment) -> LandingState {
    match commitment {
        Commitment::Processed => LandingState::Processed,
        Commitment::Confirmed => LandingState::Confirmed,
        Commitment::Finalized => LandingState::Finalized,
    }
}

const fn application_bit(outcome: BusinessOutcome) -> u8 {
    match outcome {
        BusinessOutcome::Unknown => 0,
        BusinessOutcome::Success => 1,
        BusinessOutcome::Failure => 2,
        BusinessOutcome::Timeout => 4,
        BusinessOutcome::Cancelled => 8,
    }
}

#[derive(Clone, Copy)]
struct EventIdentity {
    event: EventId,
    project: ProjectId,
    environment: EnvironmentId,
    trace: Option<TraceId>,
    business_action: Option<BusinessActionId>,
}

macro_rules! identity_match {
    ($event:expr, $binding:ident => $result:expr) => {
        match $event {
            WireEvent::TraceCreated($binding) => $result,
            WireEvent::BlockhashAcquired($binding) => $result,
            WireEvent::SimulationStarted($binding) => $result,
            WireEvent::SimulationCompleted($binding) => $result,
            WireEvent::SigningStarted($binding) => $result,
            WireEvent::SigningCompleted($binding) => $result,
            WireEvent::SubmissionStarted($binding) => $result,
            WireEvent::SubmissionCompleted($binding) => $result,
            WireEvent::SubmissionRetryScheduled($binding) => $result,
            WireEvent::ConfirmationWaitStarted($binding) => $result,
            WireEvent::ConfirmationWaitCompleted($binding) => $result,
            WireEvent::StatusObserved($binding) => $result,
            WireEvent::ExecutionEnriched($binding) => $result,
            WireEvent::BusinessOutcomeObserved($binding) => $result,
            WireEvent::DataQualityDetected($binding) => $result,
        }
    };
}

fn identity(event: &WireEvent) -> EventIdentity {
    identity_match!(event, value => EventIdentity {
        event: value.event_id,
        project: value.project_id,
        environment: value.environment_id,
        trace: value.trace_id,
        business_action: value.business_action_id,
    })
}
