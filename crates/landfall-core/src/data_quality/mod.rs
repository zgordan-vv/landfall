//! Versioned, deterministic assessment of telemetry completeness and consistency.

use std::collections::BTreeSet;

use landfall_protocol::{
    AttemptId, DataQualityCategory, DataQualitySeverity, EventId, PrivacyMode, SigningResult,
    WireEvent,
};

use crate::{
    domain::{ExecutionState, LandingState, LifecycleStage, ObservationCompleteness},
    grouping::{GroupingWarning, TraceGrouping},
    ordering::{CanonicalOrder, OrderingWarning},
    reducer::TraceProjection,
};

/// Stable identity for this scoring rubric and its contextual checks.
pub const DATA_QUALITY_VERSION: &str = "data-quality-v1";

/// Ordered quality band; higher values mean more complete and consistent evidence.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DataQualityGrade {
    /// Critical evidence gaps or several major limitations.
    F,
    /// Material limitations; many conclusions will remain unknown.
    D,
    /// Usable for bounded conclusions with visible caveats.
    C,
    /// Good coverage with only minor or isolated limitations.
    B,
    /// Strong coverage for the supported P0 analysis.
    A,
}

impl DataQualityGrade {
    /// Stable single-character representation used by projections and filters.
    #[must_use]
    pub const fn as_char(self) -> char {
        match self {
            Self::A => 'A',
            Self::B => 'B',
            Self::C => 'C',
            Self::D => 'D',
            Self::F => 'F',
        }
    }
}

/// Machine-readable limitation shown individually to users and diagnostic rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataQualityFindingCode {
    /// A producer/collector emitted an explicit quality condition.
    Reported(DataQualityCategory),
    /// Partial evidence omitted the trace creation fact.
    MissingTraceCreated,
    /// Observed later stages lack any signing evidence.
    MissingSigningEvidence,
    /// Observed later stages lack any real submission invocation.
    MissingSubmissionEvidence,
    /// A real invocation has a start but no result.
    MissingSubmissionResponse {
        /// Invocation whose completion fact is absent.
        attempt_id: AttemptId,
    },
    /// Expiration analysis needs a recent-blockhash validity boundary.
    MissingLastValidBlockHeight,
    /// A non-terminal submitted trace has no observer evidence.
    MissingObserverCoverage,
    /// Included transaction evidence has no execution enrichment.
    MissingExecutionMetadata,
    /// Simulation capture is optional but unavailable.
    MissingSimulationEvidence,
    /// Signature/fingerprint correlation is unavailable.
    MissingSignedIdentity,
    /// Strict privacy deliberately withheld signed identity correlation.
    SignedIdentityWithheldByPrivacy,
    /// Replacement grouping is unavailable without an explicit action identity.
    MissingBusinessActionCorrelation,
    /// Producer/collector clocks or cross-source ordering are uncertain.
    ClockQualityIssue,
    /// An explicit retry intent references absent completion evidence.
    MissingRetrySource {
        /// Referenced prior invocation.
        attempt_id: AttemptId,
    },
    /// One trace contains incompatible high-confidence signed identities.
    ConflictingSignedIdentity,
}

/// One deduplicated quality limitation plus all immutable evidence that reported it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataQualityFinding {
    code: DataQualityFindingCode,
    severity: DataQualitySeverity,
    evidence: BTreeSet<EventId>,
}

impl DataQualityFinding {
    /// Stable condition key.
    #[must_use]
    pub const fn code(&self) -> DataQualityFindingCode {
        self.code
    }

    /// Scoring and display priority.
    #[must_use]
    pub const fn severity(&self) -> DataQualitySeverity {
        self.severity
    }

    /// Source events supporting this condition; inferred absences may have none.
    #[must_use]
    pub fn evidence(&self) -> impl ExactSizeIterator<Item = &EventId> {
        self.evidence.iter()
    }
}

/// Complete versioned assessment, independent of transaction success.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataQualityAssessment {
    score: u8,
    grade: DataQualityGrade,
    findings: Vec<DataQualityFinding>,
}

impl DataQualityAssessment {
    /// Score in `0..=100` under [`DATA_QUALITY_VERSION`].
    #[must_use]
    pub const fn score(&self) -> u8 {
        self.score
    }

    /// Stable band derived only from the score.
    #[must_use]
    pub const fn grade(&self) -> DataQualityGrade {
        self.grade
    }

    /// Individually visible limitations in deterministic evaluation order.
    #[must_use]
    pub fn findings(&self) -> &[DataQualityFinding] {
        &self.findings
    }

    /// Rubric semantics that produced the assessment.
    #[must_use]
    pub const fn version(&self) -> &'static str {
        DATA_QUALITY_VERSION
    }
}

/// Mismatch between otherwise independently validated reducer products.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataQualityError {
    /// Projection, grouping, and canonical events do not identify one trace.
    TraceMismatch,
    /// Projection and grouping cross a project or environment boundary.
    ScopeMismatch,
}

impl std::fmt::Display for DataQualityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TraceMismatch => {
                formatter.write_str("data-quality inputs identify different traces")
            }
            Self::ScopeMismatch => {
                formatter.write_str("data-quality inputs cross project or environment boundaries")
            }
        }
    }
}

impl std::error::Error for DataQualityError {}

/// Evaluates documented required and optional evidence for one reduced trace.
pub fn evaluate_data_quality(
    events: &CanonicalOrder,
    projection: &TraceProjection,
    grouping: &TraceGrouping,
) -> Result<DataQualityAssessment, DataQualityError> {
    validate_inputs(events, projection, grouping)?;
    let mut findings = Vec::new();
    collect_reported_findings(events, &mut findings);
    collect_clock_findings(events, &mut findings);
    collect_grouping_findings(grouping, &mut findings);
    collect_contextual_findings(events, projection, grouping, &mut findings);

    let deduction = findings
        .iter()
        .map(|finding| severity_deduction(finding.severity))
        .fold(0_u8, u8::saturating_add);
    let score = 100_u8.saturating_sub(deduction);
    Ok(DataQualityAssessment {
        score,
        grade: grade_for(score),
        findings,
    })
}

fn validate_inputs(
    events: &CanonicalOrder,
    projection: &TraceProjection,
    grouping: &TraceGrouping,
) -> Result<(), DataQualityError> {
    if projection.project_id() != grouping.project_id()
        || projection.trace().environment_id() != grouping.environment_id()
    {
        return Err(DataQualityError::ScopeMismatch);
    }
    if projection.trace().id() != grouping.trace_id() {
        return Err(DataQualityError::TraceMismatch);
    }
    for collected in events.events() {
        let identity = identity(&collected.event);
        if identity.trace != Some(grouping.trace_id()) {
            return Err(DataQualityError::TraceMismatch);
        }
        if identity.project != grouping.project_id()
            || identity.environment != grouping.environment_id()
        {
            return Err(DataQualityError::ScopeMismatch);
        }
    }
    Ok(())
}

fn collect_reported_findings(events: &CanonicalOrder, findings: &mut Vec<DataQualityFinding>) {
    for collected in events.events() {
        if let WireEvent::DataQualityDetected(event) = &collected.event {
            add_finding(
                findings,
                DataQualityFindingCode::Reported(event.attributes.category),
                event.attributes.severity,
                [event.event_id],
            );
        }
    }
}

fn collect_clock_findings(events: &CanonicalOrder, findings: &mut Vec<DataQualityFinding>) {
    for warning in events.warnings() {
        match *warning {
            OrderingWarning::DuplicateDelivery { .. } => {}
            OrderingWarning::ClockSkew { event_id, .. } => add_finding(
                findings,
                DataQualityFindingCode::ClockQualityIssue,
                DataQualitySeverity::Warning,
                [event_id],
            ),
            OrderingWarning::WallClockRegression {
                earlier_event_id,
                later_event_id,
                ..
            }
            | OrderingWarning::MonotonicSemanticConflict {
                monotonic_earlier_event_id: earlier_event_id,
                semantic_earlier_event_id: later_event_id,
            } => add_finding(
                findings,
                DataQualityFindingCode::ClockQualityIssue,
                DataQualitySeverity::Warning,
                [earlier_event_id, later_event_id],
            ),
            OrderingWarning::IncomparableClockOrder {
                earlier_event_id,
                later_event_id,
            } => add_finding(
                findings,
                DataQualityFindingCode::ClockQualityIssue,
                DataQualitySeverity::Info,
                [earlier_event_id, later_event_id],
            ),
        }
    }
}

fn collect_grouping_findings(grouping: &TraceGrouping, findings: &mut Vec<DataQualityFinding>) {
    for warning in grouping.warnings() {
        match *warning {
            GroupingWarning::RetryReferencesMissingAttempt {
                attempt_id,
                event_id,
            } => add_finding(
                findings,
                DataQualityFindingCode::MissingRetrySource { attempt_id },
                DataQualitySeverity::Warning,
                [event_id],
            ),
            GroupingWarning::ConflictingSignedIdentity => add_finding(
                findings,
                DataQualityFindingCode::ConflictingSignedIdentity,
                DataQualitySeverity::Error,
                [],
            ),
        }
    }
}

fn collect_contextual_findings(
    events: &CanonicalOrder,
    projection: &TraceProjection,
    grouping: &TraceGrouping,
    findings: &mut Vec<DataQualityFinding>,
) {
    let facts = EvidenceFacts::from_events(events);
    let state = projection.trace().state();
    if !facts.has(TRACE_CREATED) {
        add_missing(
            findings,
            DataQualityFindingCode::MissingTraceCreated,
            DataQualitySeverity::Warning,
        );
    }
    if state.lifecycle >= LifecycleStage::Submitted && !facts.has(SIGNING) {
        add_missing(
            findings,
            DataQualityFindingCode::MissingSigningEvidence,
            DataQualitySeverity::Warning,
        );
    }
    if state.lifecycle >= LifecycleStage::Submitted && grouping.attempts().is_empty() {
        add_missing(
            findings,
            DataQualityFindingCode::MissingSubmissionEvidence,
            DataQualitySeverity::Warning,
        );
    }
    for attempt in grouping.attempts() {
        if attempt.evidence().completed_event_id().is_none() {
            add_missing(
                findings,
                DataQualityFindingCode::MissingSubmissionResponse {
                    attempt_id: attempt.id(),
                },
                DataQualitySeverity::Warning,
            );
        }
    }
    if state.lifecycle >= LifecycleStage::Submitted
        && !state.landing.is_landed()
        && !matches!(state.landing, LandingState::Expired)
        && !facts.has(DURABLE_NONCE)
        && !facts.has(LAST_VALID_BLOCK_HEIGHT)
    {
        add_missing(
            findings,
            DataQualityFindingCode::MissingLastValidBlockHeight,
            DataQualitySeverity::Warning,
        );
    }
    if state.lifecycle >= LifecycleStage::Submitted
        && matches!(state.observation, ObservationCompleteness::InProgress)
        && !facts.has(OBSERVER_EVIDENCE)
    {
        add_missing(
            findings,
            DataQualityFindingCode::MissingObserverCoverage,
            DataQualitySeverity::Warning,
        );
    }
    if state.landing.is_landed() && state.execution == ExecutionState::Unknown {
        add_missing(
            findings,
            DataQualityFindingCode::MissingExecutionMetadata,
            DataQualitySeverity::Warning,
        );
    }
    if !facts.has(SIMULATION) {
        add_missing(
            findings,
            DataQualityFindingCode::MissingSimulationEvidence,
            DataQualitySeverity::Info,
        );
    }
    if state.lifecycle >= LifecycleStage::Signed
        && grouping.signatures().is_empty()
        && grouping.fingerprints().is_empty()
    {
        let code = if facts.has(STRICT_PRIVACY) {
            DataQualityFindingCode::SignedIdentityWithheldByPrivacy
        } else {
            DataQualityFindingCode::MissingSignedIdentity
        };
        add_missing(findings, code, DataQualitySeverity::Info);
    }
    if grouping.business_action_id().is_none() {
        add_missing(
            findings,
            DataQualityFindingCode::MissingBusinessActionCorrelation,
            DataQualitySeverity::Info,
        );
    }
}

const TRACE_CREATED: u8 = 1 << 0;
const SIGNING: u8 = 1 << 1;
const SIMULATION: u8 = 1 << 2;
const LAST_VALID_BLOCK_HEIGHT: u8 = 1 << 3;
const DURABLE_NONCE: u8 = 1 << 4;
const OBSERVER_EVIDENCE: u8 = 1 << 5;
const STRICT_PRIVACY: u8 = 1 << 6;

#[derive(Default)]
struct EvidenceFacts {
    flags: u8,
}

impl EvidenceFacts {
    const fn has(&self, flag: u8) -> bool {
        self.flags & flag != 0
    }

    fn from_events(events: &CanonicalOrder) -> Self {
        let mut facts = Self::default();
        for collected in events.events() {
            if identity(&collected.event).privacy == PrivacyMode::Strict {
                facts.flags |= STRICT_PRIVACY;
            }
            match &collected.event {
                WireEvent::TraceCreated(event) => {
                    facts.flags |= TRACE_CREATED;
                    if event.attributes.uses_durable_nonce == Some(true) {
                        facts.flags |= DURABLE_NONCE;
                    }
                }
                WireEvent::BlockhashAcquired(event) => {
                    if event.attributes.last_valid_block_height.is_some() {
                        facts.flags |= LAST_VALID_BLOCK_HEIGHT;
                    }
                }
                WireEvent::SimulationStarted(_) | WireEvent::SimulationCompleted(_) => {
                    facts.flags |= SIMULATION;
                }
                WireEvent::SigningCompleted(event) => {
                    if event.attributes.result == SigningResult::Completed {
                        facts.flags |= SIGNING;
                    }
                }
                WireEvent::StatusObserved(_) | WireEvent::ExecutionEnriched(_) => {
                    facts.flags |= OBSERVER_EVIDENCE;
                }
                WireEvent::SigningStarted(_)
                | WireEvent::SubmissionStarted(_)
                | WireEvent::SubmissionCompleted(_)
                | WireEvent::SubmissionRetryScheduled(_)
                | WireEvent::ConfirmationWaitStarted(_)
                | WireEvent::ConfirmationWaitCompleted(_)
                | WireEvent::BusinessOutcomeObserved(_)
                | WireEvent::DataQualityDetected(_) => {}
            }
        }
        facts
    }
}

fn add_missing(
    findings: &mut Vec<DataQualityFinding>,
    code: DataQualityFindingCode,
    severity: DataQualitySeverity,
) {
    add_finding(findings, code, severity, []);
}

fn add_finding(
    findings: &mut Vec<DataQualityFinding>,
    code: DataQualityFindingCode,
    severity: DataQualitySeverity,
    evidence: impl IntoIterator<Item = EventId>,
) {
    if let Some(existing) = findings.iter_mut().find(|finding| finding.code == code) {
        if severity_rank(severity) > severity_rank(existing.severity) {
            existing.severity = severity;
        }
        existing.evidence.extend(evidence);
    } else {
        findings.push(DataQualityFinding {
            code,
            severity,
            evidence: evidence.into_iter().collect(),
        });
    }
}

const fn severity_rank(severity: DataQualitySeverity) -> u8 {
    match severity {
        DataQualitySeverity::Info => 1,
        DataQualitySeverity::Warning => 2,
        DataQualitySeverity::Error => 3,
    }
}

const fn severity_deduction(severity: DataQualitySeverity) -> u8 {
    match severity {
        DataQualitySeverity::Info => 5,
        DataQualitySeverity::Warning => 15,
        DataQualitySeverity::Error => 40,
    }
}

const fn grade_for(score: u8) -> DataQualityGrade {
    match score {
        90..=u8::MAX => DataQualityGrade::A,
        75..=89 => DataQualityGrade::B,
        60..=74 => DataQualityGrade::C,
        40..=59 => DataQualityGrade::D,
        0..=39 => DataQualityGrade::F,
    }
}

#[derive(Clone, Copy)]
struct EventIdentity {
    project: landfall_protocol::ProjectId,
    environment: landfall_protocol::EnvironmentId,
    trace: Option<landfall_protocol::TraceId>,
    privacy: PrivacyMode,
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
        project: value.project_id,
        environment: value.environment_id,
        trace: value.trace_id,
        privacy: value.privacy_mode,
    })
}
