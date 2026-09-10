//! Versioned deterministic diagnostic rules over one trace projection.

use std::collections::BTreeMap;

use landfall_protocol::{
    ConfirmationWaitResult, EventId, ExecutionResult, NormalizedErrorCategory, SimulationRpcResult,
    StatusSourceResult, SubmissionRpcResult, TransportResult, WireEvent,
};

use crate::{
    data_quality::{DataQualityFindingCode, evaluate_data_quality},
    domain::{DiagnosticId, EvidenceSet, ExecutionState, LandingState},
    grouping::TraceGrouping,
    ordering::{CanonicalOrder, CollectedEvent},
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
    unknown_reason: Option<UnknownReason>,
}

impl DiagnosticFinding {
    /// Constructs a finding for deterministic integration tests and adapters.
    #[doc(hidden)]
    #[must_use]
    pub fn test_new(
        id: DiagnosticId,
        claim_key: DiagnosticClaimKey,
        certainty: DiagnosticCertainty,
        evidence: EvidenceSet,
    ) -> Self {
        Self {
            id,
            rule_id: DiagnosticRuleId::MissingEvidence,
            rule_set_version: DIAGNOSTIC_RULE_SET_VERSION,
            claim_key,
            certainty,
            evidence,
            unknown_reason: None,
        }
    }
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

    /// Structured reason why a claim remains unknown, when applicable.
    #[must_use]
    pub const fn unknown_reason(&self) -> Option<UnknownReason> {
        self.unknown_reason
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
    /// Signing consumed an unusually large local duration.
    ExcessiveSigningDelay,
    /// Simulation consumed a high absolute amount of compute.
    LowComputeHeadroom,
    /// One route shows repeated transport or throttling failures.
    RouteDegradationSignal,
    /// A retry was scheduled after an ambiguous or unsuccessful attempt.
    UnsafeRedundantRetry,
    /// Fee was below comparable local fee observations.
    FeeLikelyUncompetitive,
    /// Required evidence is missing or materially incomplete.
    MissingEvidence,
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
            Self::ExcessiveSigningDelay => "RULE-SIGN-001",
            Self::LowComputeHeadroom => "RULE-CU-002",
            Self::RouteDegradationSignal => "RULE-ROUTE-001",
            Self::UnsafeRedundantRetry => "RULE-RETRY-001",
            Self::FeeLikelyUncompetitive => "RULE-FEE-001",
            Self::MissingEvidence => "RULE-UNKNOWN-001",
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
    /// Local signing consumed a configured excessive duration.
    ExcessiveSigningDelay,
    /// Simulation consumed a high amount of compute without direct failure.
    LowComputeHeadroom,
    /// Repeated route failures suggest degraded RPC/network service.
    RouteDegradationSignal,
    /// Retry behavior is potentially unsafe or redundant.
    UnsafeRedundantRetry,
    /// Priority fee is probably uncompetitive (requires fee-market evidence).
    FeeLikelyUncompetitive,
    /// Required evidence is missing or materially incomplete.
    MissingEvidence,
}

/// Certainty for a specific diagnostic claim.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DiagnosticCertainty {
    /// The category is directly supported by retained evidence.
    Confirmed,
    /// The evidence supports a risk signal, but does not prove causality.
    Probable,
    /// No causal claim is made because required evidence is absent.
    Unknown,
}

/// Machine-readable missing-evidence reason.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UnknownReason {
    /// Trace creation event was not retained.
    MissingTraceCreated,
    /// No signing evidence was retained.
    MissingSigningEvidence,
    /// No submission invocation was retained.
    MissingSubmissionEvidence,
    /// A submission start has no completion.
    MissingSubmissionResponse,
    /// Validity-window boundary is absent.
    MissingLastValidBlockHeight,
    /// Observer coverage is absent for a non-terminal trace.
    MissingObserverCoverage,
    /// Included transaction lacks execution metadata.
    MissingExecutionMetadata,
    /// Optional simulation evidence is absent.
    MissingSimulationEvidence,
    /// Signed identity correlation is unavailable.
    MissingSignedIdentity,
    /// Explicit business-action identity is absent.
    MissingBusinessActionCorrelation,
    /// Ordering clocks are uncertain.
    ClockQualityIssue,
    /// Observer disagreement was reported.
    ObserverDisagreement,
}

/// Conservative thresholds for the initial probable rule set.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProbableDiagnosticConfig {
    /// Signing duration at or above this value is considered excessive.
    pub excessive_signing_duration_ns: u64,
    /// Absolute simulated compute consumption considered low-headroom risk when
    /// no requested limit was retained by the current protocol version.
    pub low_compute_units_threshold: u64,
    /// Number of route failures required before emitting a degradation signal.
    pub route_failure_count_threshold: usize,
}

impl Default for ProbableDiagnosticConfig {
    fn default() -> Self {
        Self {
            excessive_signing_duration_ns: 20_000_000_000,
            low_compute_units_threshold: 90_000,
            route_failure_count_threshold: 2,
        }
    }
}

/// Evaluates probable risk signals. Missing fee-market evidence intentionally
/// produces no fee finding; a local trace cannot justify that claim.
#[must_use]
pub fn evaluate_probable_diagnostics(
    events: &CanonicalOrder,
    projection: &TraceProjection,
    grouping: &TraceGrouping,
    config: ProbableDiagnosticConfig,
) -> Vec<DiagnosticFinding> {
    let mut findings = Vec::new();
    let has_direct_compute_failure = evaluate_confirmed_diagnostics(events, projection)
        .iter()
        .any(|finding| finding.claim_key() == DiagnosticClaimKey::ComputeBudgetFailure);

    for collected in events.events() {
        match &collected.event {
            WireEvent::SigningCompleted(event)
                if event.attributes.result == landfall_protocol::SigningResult::Completed
                    && event.attributes.duration_ns.get()
                        >= config.excessive_signing_duration_ns =>
            {
                push_probable(
                    &mut findings,
                    event.event_id,
                    DiagnosticRuleId::ExcessiveSigningDelay,
                    DiagnosticClaimKey::ExcessiveSigningDelay,
                );
            }
            WireEvent::SimulationCompleted(event)
                if !has_direct_compute_failure
                    && event.attributes.rpc_result == SimulationRpcResult::Succeeded
                    && event
                        .attributes
                        .units_consumed
                        .is_some_and(|units| units.get() >= config.low_compute_units_threshold) =>
            {
                push_probable(
                    &mut findings,
                    event.event_id,
                    DiagnosticRuleId::LowComputeHeadroom,
                    DiagnosticClaimKey::LowComputeHeadroom,
                );
            }
            _ => {}
        }
    }

    let mut route_failures = BTreeMap::<landfall_protocol::RouteId, Vec<EventId>>::new();
    for collected in events.events() {
        if let WireEvent::SubmissionCompleted(event) = &collected.event {
            let degraded = matches!(
                event.attributes.transport_result,
                TransportResult::Timeout | TransportResult::ConnectionFailed
            ) || matches!(
                event.attributes.rpc_result,
                SubmissionRpcResult::RateLimited
            );
            if degraded {
                route_failures
                    .entry(event.attributes.route_id)
                    .or_default()
                    .push(event.event_id);
            }
        }
    }
    if let Some(evidence) = route_failures
        .values()
        .find(|ids| ids.len() >= config.route_failure_count_threshold)
        .and_then(|ids| ids.last())
    {
        push_probable(
            &mut findings,
            *evidence,
            DiagnosticRuleId::RouteDegradationSignal,
            DiagnosticClaimKey::RouteDegradationSignal,
        );
    }

    if grouping.has_retries()
        && events.events().iter().any(|collected| matches!(&collected.event, WireEvent::SubmissionCompleted(event) if event.attributes.transport_result != TransportResult::ResponseReceived))
        && let Some(evidence) = events.events().iter().rev().find_map(|collected| matches!(&collected.event, WireEvent::SubmissionStarted(_) | WireEvent::SubmissionCompleted(_)).then_some(collected.event_id()))
    {
        push_probable(&mut findings, evidence, DiagnosticRuleId::UnsafeRedundantRetry, DiagnosticClaimKey::UnsafeRedundantRetry);
    }
    findings
}

/// Converts data-quality gaps into explicit unknown findings.
#[must_use]
pub fn evaluate_unknown_diagnostics(
    events: &CanonicalOrder,
    projection: &TraceProjection,
    grouping: &TraceGrouping,
) -> Vec<DiagnosticFinding> {
    let Ok(assessment) = evaluate_data_quality(events, projection, grouping) else {
        return Vec::new();
    };
    let anchor = events.events().first().map(CollectedEvent::event_id);
    let Some(anchor) = anchor else {
        return Vec::new();
    };
    assessment
        .findings()
        .iter()
        .filter_map(|finding| unknown_reason(finding.code()).map(|reason| (reason, finding)))
        .filter_map(|(reason, finding)| {
            let evidence = finding.evidence().next().copied().unwrap_or(anchor);
            let Ok(id) = DiagnosticId::try_from(evidence.into_uuid()) else {
                return None;
            };
            Some(DiagnosticFinding {
                id,
                rule_id: DiagnosticRuleId::MissingEvidence,
                rule_set_version: DIAGNOSTIC_RULE_SET_VERSION,
                claim_key: DiagnosticClaimKey::MissingEvidence,
                certainty: DiagnosticCertainty::Unknown,
                evidence: EvidenceSet::new(evidence),
                unknown_reason: Some(reason),
            })
        })
        .collect()
}

fn unknown_reason(code: DataQualityFindingCode) -> Option<UnknownReason> {
    Some(match code {
        DataQualityFindingCode::MissingTraceCreated => UnknownReason::MissingTraceCreated,
        DataQualityFindingCode::MissingSigningEvidence => UnknownReason::MissingSigningEvidence,
        DataQualityFindingCode::MissingSubmissionEvidence => {
            UnknownReason::MissingSubmissionEvidence
        }
        DataQualityFindingCode::MissingSubmissionResponse { .. }
        | DataQualityFindingCode::MissingRetrySource { .. } => {
            UnknownReason::MissingSubmissionResponse
        }
        DataQualityFindingCode::MissingLastValidBlockHeight => {
            UnknownReason::MissingLastValidBlockHeight
        }
        DataQualityFindingCode::MissingObserverCoverage => UnknownReason::MissingObserverCoverage,
        DataQualityFindingCode::MissingExecutionMetadata => UnknownReason::MissingExecutionMetadata,
        DataQualityFindingCode::MissingSimulationEvidence => {
            UnknownReason::MissingSimulationEvidence
        }
        DataQualityFindingCode::MissingSignedIdentity
        | DataQualityFindingCode::SignedIdentityWithheldByPrivacy => {
            UnknownReason::MissingSignedIdentity
        }
        DataQualityFindingCode::MissingBusinessActionCorrelation => {
            UnknownReason::MissingBusinessActionCorrelation
        }
        DataQualityFindingCode::ClockQualityIssue => UnknownReason::ClockQualityIssue,
        DataQualityFindingCode::Reported(
            landfall_protocol::DataQualityCategory::ObserverDisagreement,
        )
        | DataQualityFindingCode::ConflictingSignedIdentity => UnknownReason::ObserverDisagreement,
        DataQualityFindingCode::Reported(_) => return None,
    })
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
        unknown_reason: None,
    });
}

fn push_probable(
    findings: &mut Vec<DiagnosticFinding>,
    evidence_event_id: EventId,
    rule_id: DiagnosticRuleId,
    claim_key: DiagnosticClaimKey,
) {
    let Ok(id) = DiagnosticId::try_from(evidence_event_id.into_uuid()) else {
        return;
    };
    findings.push(DiagnosticFinding {
        id,
        rule_id,
        rule_set_version: DIAGNOSTIC_RULE_SET_VERSION,
        claim_key,
        certainty: DiagnosticCertainty::Probable,
        evidence: EvidenceSet::new(evidence_event_id),
        unknown_reason: None,
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
