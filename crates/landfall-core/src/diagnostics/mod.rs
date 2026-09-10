//! Versioned deterministic diagnostic rules over one trace projection.

use landfall_protocol::{
    ConfirmationWaitResult, EventId, ExecutionResult, NormalizedErrorCategory, SimulationRpcResult,
    StatusSourceResult, SubmissionRpcResult, TransportResult, WireEvent,
};

use crate::{
    domain::{DiagnosticId, EvidenceSet, ExecutionState, LandingState},
    ordering::CanonicalOrder,
    reducer::TraceProjection,
};

/// Stable identity of the initial diagnostic rule semantics.
pub const DIAGNOSTIC_RULE_SET_VERSION: &str = "diagnostic-rules-v1";

/// One versioned deterministic diagnostic claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFinding {
    id: DiagnosticId,
    rule_id: DiagnosticRuleId,
    rule_set_version: &'static str,
    claim_key: DiagnosticClaimKey,
    certainty: DiagnosticCertainty,
    evidence: EvidenceSet,
}

impl DiagnosticFinding {
    /// Deterministic finding identity.
    #[must_use]
    pub const fn id(&self) -> DiagnosticId {
        self.id
    }

    /// Versioned rule that produced this claim.
    #[must_use]
    pub const fn rule_id(&self) -> DiagnosticRuleId {
        self.rule_id
    }

    /// Rule-set semantics used to evaluate this finding.
    #[must_use]
    pub const fn rule_set_version(&self) -> &'static str {
        self.rule_set_version
    }

    /// Machine-readable claim made by the rule.
    #[must_use]
    pub const fn claim_key(&self) -> DiagnosticClaimKey {
        self.claim_key
    }

    /// Certainty attached to this specific claim.
    #[must_use]
    pub const fn certainty(&self) -> DiagnosticCertainty {
        self.certainty
    }

    /// Immutable events supporting this claim.
    #[must_use]
    pub const fn evidence(&self) -> &EvidenceSet {
        &self.evidence
    }
}

/// Stable identifier for a deterministic diagnostic rule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticRuleId {
    /// Direct simulation failure.
    SimulationError,
    /// Structured route-level submission rejection.
    RpcSubmissionRejection,
    /// Direct on-chain execution failure.
    OnChainExecutionError,
    /// Direct compute-budget exhaustion.
    ComputeBudgetFailure,
    /// Recent-blockhash validity window passed with no observed inclusion.
    ValidityWindowPassed,
    /// Local timeout later contradicted by network success evidence.
    ClientTimeoutNetworkSuccess,
}

impl DiagnosticRuleId {
    /// Stable wire/catalog token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SimulationError => "RULE-SIM-001",
            Self::RpcSubmissionRejection => "RULE-RPC-001",
            Self::OnChainExecutionError => "RULE-EXEC-001",
            Self::ComputeBudgetFailure => "RULE-CU-001",
            Self::ValidityWindowPassed => "RULE-EXP-001",
            Self::ClientTimeoutNetworkSuccess => "RULE-TIMEOUT-001",
        }
    }
}

/// Machine-readable diagnostic category.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DiagnosticClaimKey {
    /// Simulation returned an execution/node/blockhash/malformed error.
    SimulationError,
    /// Submission route returned a structured RPC rejection.
    RpcSubmissionRejection,
    /// Included transaction failed during on-chain execution.
    OnChainExecutionError,
    /// Simulation or on-chain evidence explicitly reports compute exhaustion.
    ComputeBudgetFailure,
    /// Configured observation policy reached expiration without inclusion.
    ExpiredWithoutObservedInclusion,
    /// Client wait/submission timed out, but the transaction later landed successfully.
    ClientTimeoutFollowedByNetworkSuccess,
}

/// Certainty for a specific diagnostic claim.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticCertainty {
    /// The category is directly supported by retained evidence.
    Confirmed,
}

/// Evaluates the initial confirmed diagnostic rules for one trace.
///
/// Rules return structured claims only. Presentation wording, recommendations,
/// and probable/unknown claims are intentionally left to later rule layers.
pub fn evaluate_confirmed_diagnostics(
    events: &CanonicalOrder,
    projection: &TraceProjection,
) -> Vec<DiagnosticFinding> {
    let mut findings = Vec::new();
    let state = projection.trace().state();

    for collected in events.events() {
        match &collected.event {
            WireEvent::SimulationCompleted(event) => {
                let Some(claim_key) =
                    simulation_claim(event.attributes.rpc_result, event.attributes.error.as_ref())
                else {
                    continue;
                };
                push_finding(
                    &mut findings,
                    event.event_id,
                    match claim_key {
                        DiagnosticClaimKey::ComputeBudgetFailure => {
                            DiagnosticRuleId::ComputeBudgetFailure
                        }
                        DiagnosticClaimKey::SimulationError => DiagnosticRuleId::SimulationError,
                        _ => unreachable!("simulation rules produce only simulation claims"),
                    },
                    claim_key,
                );
            }
            WireEvent::SubmissionCompleted(event) => {
                if is_structured_submission_rejection(event.attributes.rpc_result) {
                    push_finding(
                        &mut findings,
                        event.event_id,
                        DiagnosticRuleId::RpcSubmissionRejection,
                        DiagnosticClaimKey::RpcSubmissionRejection,
                    );
                }
            }
            WireEvent::ExecutionEnriched(event)
                if event.attributes.execution_result == ExecutionResult::Failure =>
            {
                let compute_failure = event
                    .attributes
                    .error
                    .as_ref()
                    .is_some_and(is_compute_error);
                push_finding(
                    &mut findings,
                    event.event_id,
                    if compute_failure {
                        DiagnosticRuleId::ComputeBudgetFailure
                    } else {
                        DiagnosticRuleId::OnChainExecutionError
                    },
                    if compute_failure {
                        DiagnosticClaimKey::ComputeBudgetFailure
                    } else {
                        DiagnosticClaimKey::OnChainExecutionError
                    },
                );
            }
            _ => {}
        }
    }

    if state.landing == LandingState::Expired
        && let Some(event_id) = latest_not_found_evidence(events)
    {
        push_finding(
            &mut findings,
            event_id,
            DiagnosticRuleId::ValidityWindowPassed,
            DiagnosticClaimKey::ExpiredWithoutObservedInclusion,
        );
    }

    if state.execution == ExecutionState::Success
        && let Some(event_id) = timeout_before_success_evidence(events)
    {
        push_finding(
            &mut findings,
            event_id,
            DiagnosticRuleId::ClientTimeoutNetworkSuccess,
            DiagnosticClaimKey::ClientTimeoutFollowedByNetworkSuccess,
        );
    }

    findings
}

fn push_finding(
    findings: &mut Vec<DiagnosticFinding>,
    evidence_event_id: EventId,
    rule_id: DiagnosticRuleId,
    claim_key: DiagnosticClaimKey,
) {
    let Ok(id) = DiagnosticId::try_from(evidence_event_id.into_uuid()) else {
        return;
    };
    let evidence = EvidenceSet::new(evidence_event_id);
    findings.push(DiagnosticFinding {
        id,
        rule_id,
        rule_set_version: DIAGNOSTIC_RULE_SET_VERSION,
        claim_key,
        certainty: DiagnosticCertainty::Confirmed,
        evidence,
    });
}

fn simulation_claim(
    result: SimulationRpcResult,
    error: Option<&landfall_protocol::NormalizedError>,
) -> Option<DiagnosticClaimKey> {
    match result {
        SimulationRpcResult::Succeeded | SimulationRpcResult::NotObserved => None,
        SimulationRpcResult::ExecutionError
        | SimulationRpcResult::BlockhashNotFound
        | SimulationRpcResult::NodeError
        | SimulationRpcResult::MalformedResponse
            if error.is_some_and(is_compute_error) =>
        {
            Some(DiagnosticClaimKey::ComputeBudgetFailure)
        }
        SimulationRpcResult::ExecutionError
        | SimulationRpcResult::BlockhashNotFound
        | SimulationRpcResult::NodeError
        | SimulationRpcResult::MalformedResponse => Some(DiagnosticClaimKey::SimulationError),
    }
}

const fn is_structured_submission_rejection(result: SubmissionRpcResult) -> bool {
    matches!(
        result,
        SubmissionRpcResult::Rejected
            | SubmissionRpcResult::BlockhashRejected
            | SubmissionRpcResult::DuplicateSignature
            | SubmissionRpcResult::AlreadyProcessed
            | SubmissionRpcResult::RateLimited
            | SubmissionRpcResult::Unauthorized
            | SubmissionRpcResult::MalformedResponse
    )
}

fn is_compute_error(error: &landfall_protocol::NormalizedError) -> bool {
    error.category == NormalizedErrorCategory::ComputeBudgetExceeded
}

fn latest_not_found_evidence(events: &CanonicalOrder) -> Option<EventId> {
    events.events().iter().rev().find_map(|collected| {
        if let WireEvent::StatusObserved(event) = &collected.event
            && event.attributes.source_result == StatusSourceResult::NotFound
        {
            return Some(event.event_id);
        }
        None
    })
}

fn timeout_before_success_evidence(events: &CanonicalOrder) -> Option<EventId> {
    let mut timeout = None;
    for collected in events.events() {
        match &collected.event {
            WireEvent::SubmissionCompleted(event)
                if event.attributes.transport_result == TransportResult::Timeout =>
            {
                timeout.get_or_insert(event.event_id);
            }
            WireEvent::ConfirmationWaitCompleted(event)
                if event.attributes.result == ConfirmationWaitResult::Timeout =>
            {
                timeout.get_or_insert(event.event_id);
            }
            WireEvent::StatusObserved(event)
                if event.attributes.source_result == StatusSourceResult::Found =>
            {
                if let Some(event_id) = timeout {
                    return Some(event_id);
                }
            }
            WireEvent::ExecutionEnriched(event)
                if event.attributes.execution_result == ExecutionResult::Success =>
            {
                if let Some(event_id) = timeout {
                    return Some(event_id);
                }
            }
            _ => {}
        }
    }
    None
}
