//! Evidence-backed domain entities and their relationship boundaries.

use std::collections::BTreeSet;

use landfall_protocol::{
    AttemptId, BusinessActionId, EnvironmentId, EventId, ObserverSourceId, TraceId,
};

use super::{
    CohortId, DiagnosticId, DomainInvariantError, EvidenceSet, ExecutionMetadataId,
    LifecycleEvidence, RecommendationId, SimulationId, StateInvariantError, StatusObservationId,
    TraceState,
};

/// One explicit customer intention that may contain replacement traces.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BusinessAction {
    id: BusinessActionId,
    environment_id: EnvironmentId,
    trace_ids: BTreeSet<TraceId>,
    evidence: EvidenceSet,
}

impl BusinessAction {
    /// Creates an action from its first immutable evidence event.
    #[must_use]
    pub fn new(id: BusinessActionId, environment_id: EnvironmentId, source_event: EventId) -> Self {
        Self {
            id,
            environment_id,
            trace_ids: BTreeSet::new(),
            evidence: EvidenceSet::new(source_event),
        }
    }

    /// Links an explicitly correlated trace after checking tenant boundaries.
    pub fn link_trace(&mut self, trace: &TransactionTrace) -> Result<bool, DomainInvariantError> {
        if self.environment_id != trace.environment_id {
            return Err(DomainInvariantError::EnvironmentMismatch);
        }
        if trace.business_action_id != Some(self.id) {
            return Err(DomainInvariantError::BusinessActionMismatch);
        }
        Ok(self.trace_ids.insert(trace.id))
    }

    /// Business-action identity.
    #[must_use]
    pub const fn id(&self) -> BusinessActionId {
        self.id
    }

    /// Owning privacy and correlation boundary.
    #[must_use]
    pub const fn environment_id(&self) -> EnvironmentId {
        self.environment_id
    }

    /// Explicitly linked replacement traces.
    #[must_use]
    pub fn trace_ids(&self) -> impl ExactSizeIterator<Item = &TraceId> {
        self.trace_ids.iter()
    }

    /// Immutable events supporting this action projection.
    #[must_use]
    pub const fn evidence(&self) -> &EvidenceSet {
        &self.evidence
    }
}

/// One unique signed-transaction lifecycle, including identical-byte retries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionTrace {
    id: TraceId,
    environment_id: EnvironmentId,
    business_action_id: Option<BusinessActionId>,
    evidence: EvidenceSet,
    state: TraceState,
}

impl TransactionTrace {
    /// Creates a trace projection from its first immutable event.
    #[must_use]
    pub fn new(
        id: TraceId,
        environment_id: EnvironmentId,
        business_action_id: Option<BusinessActionId>,
        source_event: EventId,
    ) -> Self {
        Self {
            id,
            environment_id,
            business_action_id,
            evidence: EvidenceSet::new(source_event),
            state: TraceState::default(),
        }
    }

    /// Creates a trace with an already reduced, validated state.
    pub fn with_state(
        id: TraceId,
        environment_id: EnvironmentId,
        business_action_id: Option<BusinessActionId>,
        evidence: EvidenceSet,
        state: TraceState,
    ) -> Result<Self, StateInvariantError> {
        state.validate()?;
        Ok(Self {
            id,
            environment_id,
            business_action_id,
            evidence,
            state,
        })
    }

    /// Adds another source event, deduplicating transport replays by event ID.
    pub fn add_evidence(&mut self, event_id: EventId) -> bool {
        self.evidence.insert(event_id)
    }

    /// Trace identity.
    #[must_use]
    pub const fn id(&self) -> TraceId {
        self.id
    }

    /// Owning privacy and correlation boundary.
    #[must_use]
    pub const fn environment_id(&self) -> EnvironmentId {
        self.environment_id
    }

    /// Explicit customer intention, when instrumented.
    #[must_use]
    pub const fn business_action_id(&self) -> Option<BusinessActionId> {
        self.business_action_id
    }

    /// Immutable source events supporting this trace projection.
    #[must_use]
    pub const fn evidence(&self) -> &EvidenceSet {
        &self.evidence
    }

    /// Current independently modeled state dimensions.
    #[must_use]
    pub const fn state(&self) -> TraceState {
        self.state
    }
}

/// One simulation invocation correlated by a dedicated operation identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Simulation {
    id: SimulationId,
    trace_id: TraceId,
    evidence: LifecycleEvidence,
}

impl Simulation {
    /// Creates a simulation observed at its start, completion, or both.
    #[must_use]
    pub const fn new(id: SimulationId, trace_id: TraceId, evidence: LifecycleEvidence) -> Self {
        Self {
            id,
            trace_id,
            evidence,
        }
    }

    /// Simulation identity.
    #[must_use]
    pub const fn id(self) -> SimulationId {
        self.id
    }

    /// Owning transaction trace.
    #[must_use]
    pub const fn trace_id(self) -> TraceId {
        self.trace_id
    }

    /// Captured lifecycle boundaries.
    #[must_use]
    pub const fn evidence(self) -> LifecycleEvidence {
        self.evidence
    }
}

/// One real application-visible submission invocation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SubmissionAttempt {
    id: AttemptId,
    trace_id: TraceId,
    evidence: LifecycleEvidence,
}

impl SubmissionAttempt {
    /// Creates an attempt observed at its start, completion, or both.
    #[must_use]
    pub const fn new(id: AttemptId, trace_id: TraceId, evidence: LifecycleEvidence) -> Self {
        Self {
            id,
            trace_id,
            evidence,
        }
    }

    /// Submission-attempt identity.
    #[must_use]
    pub const fn id(self) -> AttemptId {
        self.id
    }

    /// Owning transaction trace.
    #[must_use]
    pub const fn trace_id(self) -> TraceId {
        self.trace_id
    }

    /// Captured lifecycle boundaries.
    #[must_use]
    pub const fn evidence(self) -> LifecycleEvidence {
        self.evidence
    }
}

/// One immutable status response from one configured observer source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StatusObservation {
    id: StatusObservationId,
    trace_id: TraceId,
    observer_source_id: ObserverSourceId,
}

impl StatusObservation {
    /// Creates an observation whose identity is its immutable source event.
    #[must_use]
    pub const fn new(
        trace_id: TraceId,
        observer_source_id: ObserverSourceId,
        source_event_id: EventId,
    ) -> Self {
        Self {
            id: StatusObservationId::new(source_event_id),
            trace_id,
            observer_source_id,
        }
    }

    /// Observation identity.
    #[must_use]
    pub const fn id(self) -> StatusObservationId {
        self.id
    }

    /// Owning transaction trace.
    #[must_use]
    pub const fn trace_id(self) -> TraceId {
        self.trace_id
    }

    /// Configured observer that produced the fact.
    #[must_use]
    pub const fn observer_source_id(self) -> ObserverSourceId {
        self.observer_source_id
    }
}

/// One immutable on-chain execution enrichment.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutionMetadata {
    id: ExecutionMetadataId,
    trace_id: TraceId,
    observer_source_id: ObserverSourceId,
}

impl ExecutionMetadata {
    /// Creates metadata whose identity is its immutable enrichment event.
    #[must_use]
    pub const fn new(
        trace_id: TraceId,
        observer_source_id: ObserverSourceId,
        source_event_id: EventId,
    ) -> Self {
        Self {
            id: ExecutionMetadataId::new(source_event_id),
            trace_id,
            observer_source_id,
        }
    }

    /// Enrichment identity.
    #[must_use]
    pub const fn id(self) -> ExecutionMetadataId {
        self.id
    }

    /// Owning transaction trace.
    #[must_use]
    pub const fn trace_id(self) -> TraceId {
        self.trace_id
    }

    /// Configured observer that produced the enrichment.
    #[must_use]
    pub const fn observer_source_id(self) -> ObserverSourceId {
        self.observer_source_id
    }
}

/// Scope of a diagnosis or recommendation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AnalysisScope {
    /// One canonical transaction trace.
    Trace(TraceId),
    /// A bounded analytical cohort.
    Cohort(CohortId),
}

/// One deterministic, reviewable claim backed by immutable events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    id: DiagnosticId,
    scope: AnalysisScope,
    evidence: EvidenceSet,
}

impl Diagnostic {
    /// Creates an evidence-backed diagnostic identity and scope.
    #[must_use]
    pub const fn new(id: DiagnosticId, scope: AnalysisScope, evidence: EvidenceSet) -> Self {
        Self {
            id,
            scope,
            evidence,
        }
    }

    /// Diagnostic identity.
    #[must_use]
    pub const fn id(&self) -> DiagnosticId {
        self.id
    }

    /// Trace or cohort whose claim is being made.
    #[must_use]
    pub const fn scope(&self) -> AnalysisScope {
        self.scope
    }

    /// Immutable events selected as supporting evidence.
    #[must_use]
    pub const fn evidence(&self) -> &EvidenceSet {
        &self.evidence
    }
}

/// One advisory output that remains explicitly linked to evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Recommendation {
    id: RecommendationId,
    scope: AnalysisScope,
    diagnostic_ids: BTreeSet<DiagnosticId>,
    evidence: EvidenceSet,
}

impl Recommendation {
    /// Creates an advisory result without granting it mutation capabilities.
    #[must_use]
    pub fn new(
        id: RecommendationId,
        scope: AnalysisScope,
        diagnostic_ids: impl IntoIterator<Item = DiagnosticId>,
        evidence: EvidenceSet,
    ) -> Self {
        Self {
            id,
            scope,
            diagnostic_ids: diagnostic_ids.into_iter().collect(),
            evidence,
        }
    }

    /// Recommendation identity.
    #[must_use]
    pub const fn id(&self) -> RecommendationId {
        self.id
    }

    /// Trace or cohort to which the advice applies.
    #[must_use]
    pub const fn scope(&self) -> AnalysisScope {
        self.scope
    }

    /// Diagnoses that led to this recommendation.
    #[must_use]
    pub fn diagnostic_ids(&self) -> impl ExactSizeIterator<Item = &DiagnosticId> {
        self.diagnostic_ids.iter()
    }

    /// Immutable events that make the advice reviewable.
    #[must_use]
    pub const fn evidence(&self) -> &EvidenceSet {
        &self.evidence
    }
}
