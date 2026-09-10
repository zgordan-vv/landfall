//! Deterministic retry, replacement, and signed-identity grouping rules.

use std::collections::{BTreeMap, BTreeSet};

use landfall_protocol::{
    AttemptId, BusinessActionId, EnvironmentId, EventId, FingerprintHex, FingerprintKeyId,
    ProjectId, RouteId, SolanaSignature, TraceId, WireEvent,
};

use crate::{
    domain::{LifecycleEvidence, SubmissionAttempt},
    ordering::CanonicalOrder,
};

/// One comparable HMAC fingerprint without exposing transaction bytes or keys.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FingerprintEvidence {
    /// Environment fingerprint-key version used for this value.
    pub key_id: FingerprintKeyId,
    /// HMAC of the exact serialized signed transaction bytes.
    pub value: FingerprintHex,
}

/// Attempts and trusted correlation evidence belonging to one trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceGrouping {
    project_id: ProjectId,
    environment_id: EnvironmentId,
    trace_id: TraceId,
    business_action_id: Option<BusinessActionId>,
    attempts: Vec<SubmissionAttempt>,
    scheduled_retry_count: usize,
    signatures: BTreeSet<SolanaSignature>,
    fingerprints: BTreeSet<FingerprintEvidence>,
    identity_conflict: bool,
    warnings: Vec<GroupingWarning>,
}

impl TraceGrouping {
    /// Owning project boundary.
    #[must_use]
    pub const fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// Owning environment and cluster boundary.
    #[must_use]
    pub const fn environment_id(&self) -> EnvironmentId {
        self.environment_id
    }

    /// Unique signed-transaction lifecycle identity supplied by instrumentation.
    #[must_use]
    pub const fn trace_id(&self) -> TraceId {
        self.trace_id
    }

    /// Explicit customer intention, when supplied.
    #[must_use]
    pub const fn business_action_id(&self) -> Option<BusinessActionId> {
        self.business_action_id
    }

    /// Actual submission invocations in canonical first-observation order.
    #[must_use]
    pub fn attempts(&self) -> &[SubmissionAttempt] {
        &self.attempts
    }

    /// Retry intents, which do not become attempts until a submission starts.
    #[must_use]
    pub const fn scheduled_retry_count(&self) -> usize {
        self.scheduled_retry_count
    }

    /// Whether more than one actual submission invocation used this trace.
    #[must_use]
    pub fn has_retries(&self) -> bool {
        self.attempts.len() > 1
    }

    /// Permitted public signatures retained as high-confidence identity evidence.
    #[must_use]
    pub const fn signatures(&self) -> &BTreeSet<SolanaSignature> {
        &self.signatures
    }

    /// Versioned signed-byte fingerprints retained as high-confidence evidence.
    #[must_use]
    pub const fn fingerprints(&self) -> &BTreeSet<FingerprintEvidence> {
        &self.fingerprints
    }

    /// Incomplete correlation facts that do not justify inventing an attempt.
    #[must_use]
    pub fn warnings(&self) -> &[GroupingWarning] {
        &self.warnings
    }
}

/// Non-fatal missing evidence found while rebuilding attempt groups.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupingWarning {
    /// A retry intent references an attempt absent from retained trace evidence.
    RetryReferencesMissingAttempt {
        /// Referenced previous invocation.
        attempt_id: AttemptId,
        /// Retry-scheduling fact carrying the reference.
        event_id: EventId,
    },
    /// One trace contains incompatible signatures or same-key fingerprints.
    ConflictingSignedIdentity,
}

/// Relationship between two already validated trace groups.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceRelationship {
    /// Both inputs describe the same explicit trace identity.
    SameTrace,
    /// Different trace IDs have trusted proof of the same signed transaction.
    SameSignedTransactionAliasCandidate,
    /// Different signed-transaction traces serve the same explicit customer intention.
    Replacement,
    /// Explicit action identities show that the traces are unrelated.
    Unrelated,
    /// No explicit business-action correlation exists; no relationship is guessed.
    GroupingUnavailable,
    /// Signature, fingerprint, or business-action evidence contradicts itself.
    ConflictingIdentityEvidence,
}

/// Evidence defect that makes attempt grouping unsafe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupingError {
    /// Attempt grouping requires at least one trace event.
    EmptyInput,
    /// A project-scoped or action-only event was supplied as trace evidence.
    MissingTraceId {
        /// Event outside trace scope.
        event_id: EventId,
    },
    /// Events for different projects were mixed into one trace group.
    ProjectMismatch {
        /// Conflicting event.
        event_id: EventId,
    },
    /// Events for different environments were mixed into one trace group.
    EnvironmentMismatch {
        /// Conflicting event.
        event_id: EventId,
    },
    /// Events for different traces were mixed into one trace group.
    TraceMismatch {
        /// Conflicting event.
        event_id: EventId,
    },
    /// One trace was assigned two explicit business actions.
    BusinessActionMismatch {
        /// Conflicting event.
        event_id: EventId,
    },
    /// One attempt has more than one start fact.
    DuplicateAttemptStart {
        /// Reused attempt identity.
        attempt_id: AttemptId,
    },
    /// One attempt has more than one completion fact.
    DuplicateAttemptCompletion {
        /// Reused attempt identity.
        attempt_id: AttemptId,
    },
    /// Start and completion assign different routes to one invocation.
    AttemptRouteMismatch {
        /// Inconsistent attempt identity.
        attempt_id: AttemptId,
    },
    /// Internal attempt evidence could not form a non-empty lifecycle entity.
    InvalidAttemptEvidence {
        /// Invalid attempt identity.
        attempt_id: AttemptId,
    },
    /// Pairwise correlation was attempted across project/environment boundaries.
    CorrelationScopeMismatch,
}

impl std::fmt::Display for GroupingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("attempt grouping requires trace evidence"),
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
            Self::DuplicateAttemptStart { attempt_id } => {
                write!(formatter, "attempt {attempt_id} has multiple start events")
            }
            Self::DuplicateAttemptCompletion { attempt_id } => write!(
                formatter,
                "attempt {attempt_id} has multiple completion events"
            ),
            Self::AttemptRouteMismatch { attempt_id } => {
                write!(formatter, "attempt {attempt_id} has conflicting routes")
            }
            Self::InvalidAttemptEvidence { attempt_id } => {
                write!(
                    formatter,
                    "attempt {attempt_id} has invalid lifecycle evidence"
                )
            }
            Self::CorrelationScopeMismatch => formatter
                .write_str("trace correlation cannot cross project or environment boundaries"),
        }
    }
}

impl std::error::Error for GroupingError {}

/// Rebuilds actual submission attempts and trusted identity evidence for one trace.
pub fn group_trace(events: &CanonicalOrder) -> Result<TraceGrouping, GroupingError> {
    let first = events.events().first().ok_or(GroupingError::EmptyInput)?;
    let base = identity(&first.event);
    let trace_id = base.trace.ok_or(GroupingError::MissingTraceId {
        event_id: base.event,
    })?;
    let mut business_action_id = base.business_action;
    let mut attempts = Vec::<AttemptBuilder>::new();
    let mut attempt_indices = BTreeMap::<AttemptId, usize>::new();
    let mut retry_references = Vec::<(AttemptId, EventId)>::new();
    let mut signatures = BTreeSet::new();
    let mut fingerprints = BTreeSet::new();

    for collected in events.events() {
        let current = identity(&collected.event);
        validate_identity(base, trace_id, business_action_id, current)?;
        if business_action_id.is_none() {
            business_action_id = current.business_action;
        }
        collect_signed_identity(&collected.event, &mut signatures, &mut fingerprints);

        collect_attempt_event(
            &collected.event,
            &mut attempts,
            &mut attempt_indices,
            &mut retry_references,
        )?;
    }

    let completed_attempts = attempts
        .iter()
        .filter(|attempt| attempt.completed_event_id.is_some())
        .map(|attempt| attempt.id)
        .collect::<BTreeSet<_>>();
    let identity_conflict = signatures.len() > 1 || conflicting_fingerprints(&fingerprints);
    let mut warnings = retry_references
        .iter()
        .filter_map(|&(attempt_id, event_id)| {
            (!completed_attempts.contains(&attempt_id)).then_some(
                GroupingWarning::RetryReferencesMissingAttempt {
                    attempt_id,
                    event_id,
                },
            )
        })
        .collect::<Vec<_>>();
    if identity_conflict {
        warnings.push(GroupingWarning::ConflictingSignedIdentity);
    }
    let attempts = build_attempts(attempts, trace_id)?;

    Ok(TraceGrouping {
        project_id: base.project,
        environment_id: base.environment,
        trace_id,
        business_action_id,
        attempts,
        scheduled_retry_count: retry_references.len(),
        signatures,
        fingerprints,
        identity_conflict,
        warnings,
    })
}

fn collect_attempt_event(
    event: &WireEvent,
    attempts: &mut Vec<AttemptBuilder>,
    attempt_indices: &mut BTreeMap<AttemptId, usize>,
    retry_references: &mut Vec<(AttemptId, EventId)>,
) -> Result<(), GroupingError> {
    match event {
        WireEvent::SubmissionStarted(event) => {
            let builder = attempt_builder(attempts, attempt_indices, event.attributes.attempt_id);
            if builder.started_event_id.is_some() {
                return Err(GroupingError::DuplicateAttemptStart {
                    attempt_id: event.attributes.attempt_id,
                });
            }
            validate_route(builder, event.attributes.route_id)?;
            builder.started_event_id = Some(event.event_id);
            builder.route_id = Some(event.attributes.route_id);
        }
        WireEvent::SubmissionCompleted(event) => {
            let builder = attempt_builder(attempts, attempt_indices, event.attributes.attempt_id);
            if builder.completed_event_id.is_some() {
                return Err(GroupingError::DuplicateAttemptCompletion {
                    attempt_id: event.attributes.attempt_id,
                });
            }
            validate_route(builder, event.attributes.route_id)?;
            builder.completed_event_id = Some(event.event_id);
            builder.route_id = Some(event.attributes.route_id);
        }
        WireEvent::SubmissionRetryScheduled(event) => {
            retry_references.push((event.attributes.previous_attempt_id, event.event_id));
        }
        WireEvent::TraceCreated(_)
        | WireEvent::BlockhashAcquired(_)
        | WireEvent::SimulationStarted(_)
        | WireEvent::SimulationCompleted(_)
        | WireEvent::SigningStarted(_)
        | WireEvent::SigningCompleted(_)
        | WireEvent::ConfirmationWaitStarted(_)
        | WireEvent::ConfirmationWaitCompleted(_)
        | WireEvent::StatusObserved(_)
        | WireEvent::ExecutionEnriched(_)
        | WireEvent::BusinessOutcomeObserved(_)
        | WireEvent::DataQualityDetected(_) => {}
    }
    Ok(())
}

fn build_attempts(
    attempts: Vec<AttemptBuilder>,
    trace_id: TraceId,
) -> Result<Vec<SubmissionAttempt>, GroupingError> {
    attempts
        .into_iter()
        .map(|attempt| {
            let evidence =
                LifecycleEvidence::new(attempt.started_event_id, attempt.completed_event_id)
                    .map_err(|_| GroupingError::InvalidAttemptEvidence {
                        attempt_id: attempt.id,
                    })?;
            Ok(SubmissionAttempt::new(attempt.id, trace_id, evidence))
        })
        .collect()
}

/// Classifies two trace identities without timing or transaction-shape heuristics.
pub fn classify_trace_relationship(
    left: &TraceGrouping,
    right: &TraceGrouping,
) -> Result<TraceRelationship, GroupingError> {
    if left.project_id != right.project_id || left.environment_id != right.environment_id {
        return Err(GroupingError::CorrelationScopeMismatch);
    }
    if left.identity_conflict || right.identity_conflict {
        return Ok(TraceRelationship::ConflictingIdentityEvidence);
    }
    if left.trace_id == right.trace_id {
        return Ok(if conflicting_actions(left, right) {
            TraceRelationship::ConflictingIdentityEvidence
        } else {
            TraceRelationship::SameTrace
        });
    }

    let signatures_equal = intersects(&left.signatures, &right.signatures);
    let signatures_differ =
        !left.signatures.is_empty() && !right.signatures.is_empty() && !signatures_equal;
    let fingerprint_comparison = compare_fingerprints(&left.fingerprints, &right.fingerprints);
    let same_signed_identity = signatures_equal || fingerprint_comparison.same;
    let identity_conflict = (signatures_equal || fingerprint_comparison.same)
        && fingerprint_comparison.different
        || (fingerprint_comparison.same && signatures_differ);

    if identity_conflict || (same_signed_identity && conflicting_actions(left, right)) {
        return Ok(TraceRelationship::ConflictingIdentityEvidence);
    }
    if same_signed_identity {
        return Ok(TraceRelationship::SameSignedTransactionAliasCandidate);
    }
    match (left.business_action_id, right.business_action_id) {
        (Some(left), Some(right)) if left == right => Ok(TraceRelationship::Replacement),
        (Some(_), Some(_)) => Ok(TraceRelationship::Unrelated),
        (None, _) | (_, None) => Ok(TraceRelationship::GroupingUnavailable),
    }
}

fn conflicting_fingerprints(fingerprints: &BTreeSet<FingerprintEvidence>) -> bool {
    let mut by_key = BTreeMap::<FingerprintKeyId, &FingerprintHex>::new();
    for fingerprint in fingerprints {
        if let Some(existing) = by_key.insert(fingerprint.key_id, &fingerprint.value)
            && existing != &fingerprint.value
        {
            return true;
        }
    }
    false
}

fn conflicting_actions(left: &TraceGrouping, right: &TraceGrouping) -> bool {
    matches!(
        (left.business_action_id, right.business_action_id),
        (Some(left), Some(right)) if left != right
    )
}

fn intersects<T: Ord>(left: &BTreeSet<T>, right: &BTreeSet<T>) -> bool {
    left.iter().any(|value| right.contains(value))
}

#[derive(Default)]
struct FingerprintComparison {
    same: bool,
    different: bool,
}

fn compare_fingerprints(
    left: &BTreeSet<FingerprintEvidence>,
    right: &BTreeSet<FingerprintEvidence>,
) -> FingerprintComparison {
    let mut comparison = FingerprintComparison::default();
    for left in left {
        for right in right {
            if left.key_id == right.key_id {
                if left.value == right.value {
                    comparison.same = true;
                } else {
                    comparison.different = true;
                }
            }
        }
    }
    comparison
}

fn collect_signed_identity(
    event: &WireEvent,
    signatures: &mut BTreeSet<SolanaSignature>,
    fingerprints: &mut BTreeSet<FingerprintEvidence>,
) {
    match event {
        WireEvent::SigningCompleted(event) => {
            if let Some(signature) = &event.attributes.signature {
                signatures.insert(signature.clone());
            }
            if let Some(fingerprint) = &event.attributes.signed_bytes_fingerprint {
                fingerprints.insert(FingerprintEvidence {
                    key_id: fingerprint.key_id,
                    value: fingerprint.value_hex.clone(),
                });
            }
        }
        WireEvent::SubmissionCompleted(event) => {
            if let Some(signature) = &event.attributes.signature {
                signatures.insert(signature.clone());
            }
        }
        WireEvent::StatusObserved(event) => {
            if let Some(signature) = &event.attributes.signature {
                signatures.insert(signature.clone());
            }
        }
        WireEvent::ExecutionEnriched(event) => {
            if let Some(signature) = &event.attributes.signature {
                signatures.insert(signature.clone());
            }
        }
        WireEvent::TraceCreated(_)
        | WireEvent::BlockhashAcquired(_)
        | WireEvent::SimulationStarted(_)
        | WireEvent::SimulationCompleted(_)
        | WireEvent::SigningStarted(_)
        | WireEvent::SubmissionStarted(_)
        | WireEvent::SubmissionRetryScheduled(_)
        | WireEvent::ConfirmationWaitStarted(_)
        | WireEvent::ConfirmationWaitCompleted(_)
        | WireEvent::BusinessOutcomeObserved(_)
        | WireEvent::DataQualityDetected(_) => {}
    }
}

#[derive(Clone, Copy)]
struct AttemptBuilder {
    id: AttemptId,
    route_id: Option<RouteId>,
    started_event_id: Option<EventId>,
    completed_event_id: Option<EventId>,
}

fn attempt_builder<'a>(
    attempts: &'a mut Vec<AttemptBuilder>,
    indices: &mut BTreeMap<AttemptId, usize>,
    attempt_id: AttemptId,
) -> &'a mut AttemptBuilder {
    let index = *indices.entry(attempt_id).or_insert_with(|| {
        let index = attempts.len();
        attempts.push(AttemptBuilder {
            id: attempt_id,
            route_id: None,
            started_event_id: None,
            completed_event_id: None,
        });
        index
    });
    &mut attempts[index]
}

fn validate_route(builder: &AttemptBuilder, route_id: RouteId) -> Result<(), GroupingError> {
    if builder
        .route_id
        .is_some_and(|existing| existing != route_id)
    {
        Err(GroupingError::AttemptRouteMismatch {
            attempt_id: builder.id,
        })
    } else {
        Ok(())
    }
}

fn validate_identity(
    expected: EventIdentity,
    trace_id: TraceId,
    business_action_id: Option<BusinessActionId>,
    current: EventIdentity,
) -> Result<(), GroupingError> {
    if current.project != expected.project {
        return Err(GroupingError::ProjectMismatch {
            event_id: current.event,
        });
    }
    if current.environment != expected.environment {
        return Err(GroupingError::EnvironmentMismatch {
            event_id: current.event,
        });
    }
    match current.trace {
        None => {
            return Err(GroupingError::MissingTraceId {
                event_id: current.event,
            });
        }
        Some(current_trace) if current_trace != trace_id => {
            return Err(GroupingError::TraceMismatch {
                event_id: current.event,
            });
        }
        Some(_) => {}
    }
    if let (Some(expected_action), Some(current_action)) =
        (business_action_id, current.business_action)
        && expected_action != current_action
    {
        return Err(GroupingError::BusinessActionMismatch {
            event_id: current.event,
        });
    }
    Ok(())
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
