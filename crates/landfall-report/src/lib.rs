//! Structured report models and portable report rendering adapter.

use landfall_core::{domain::TraceState, versions::CoreVersionManifest};
use landfall_protocol::TraceId;

/// Stable report-domain document independent of presentation formats.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportDocument {
    /// Semantics used to produce this document.
    pub versions: CoreVersionManifest,
    /// Per-trace report records.
    pub traces: Vec<TraceReport>,
    /// Number of cross-trace alias/replacement relationships.
    pub alias_count: usize,
}

/// Presentation-neutral summary for one trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceReport {
    /// Trace identity.
    pub trace_id: TraceId,
    /// Stable lifecycle state token.
    pub lifecycle: &'static str,
    /// Stable landing state token.
    pub landing: &'static str,
    /// Stable execution state token.
    pub execution: &'static str,
    /// Stable application outcome token.
    pub application: &'static str,
    /// Whether terminal metrics may include this trace.
    pub terminal_eligible: bool,
    /// Quality findings count.
    pub data_quality_findings: usize,
    /// Diagnostic counts by certainty.
    pub confirmed_diagnostics: usize,
    /// Probable diagnostics count.
    pub probable_diagnostics: usize,
    /// Unknown diagnostics count.
    pub unknown_diagnostics: usize,
    /// Advisory recommendation count.
    pub recommendations: usize,
}

impl TraceReport {
    /// Builds a stable summary from a validated state and pipeline counts.
    #[must_use]
    pub fn from_state(trace_id: TraceId, state: TraceState, counts: ReportCounts) -> Self {
        Self {
            trace_id,
            lifecycle: match state.lifecycle {
                landfall_core::domain::LifecycleStage::Created => "created",
                landfall_core::domain::LifecycleStage::Signed => "signed",
                landfall_core::domain::LifecycleStage::Submitted => "submitted",
                landfall_core::domain::LifecycleStage::Observed => "observed",
            },
            landing: match state.landing {
                landfall_core::domain::LandingState::NotObserved => "not_observed",
                landfall_core::domain::LandingState::Processed => "processed",
                landfall_core::domain::LandingState::Confirmed => "confirmed",
                landfall_core::domain::LandingState::Finalized => "finalized",
                landfall_core::domain::LandingState::Expired => "expired",
                landfall_core::domain::LandingState::Incomplete => "incomplete",
                landfall_core::domain::LandingState::Conflicting => "conflicting",
            },
            execution: match state.execution {
                landfall_core::domain::ExecutionState::Unknown => "unknown",
                landfall_core::domain::ExecutionState::Success => "success",
                landfall_core::domain::ExecutionState::Failure => "failure",
            },
            application: match state.application {
                landfall_core::domain::ApplicationOutcome::Unknown => "unknown",
                landfall_core::domain::ApplicationOutcome::Success => "success",
                landfall_core::domain::ApplicationOutcome::Failure => "failure",
                landfall_core::domain::ApplicationOutcome::Timeout => "timeout",
                landfall_core::domain::ApplicationOutcome::Cancelled => "cancelled",
            },
            terminal_eligible: state.observation.is_metric_eligible(),
            data_quality_findings: counts.data_quality_findings,
            confirmed_diagnostics: counts.confirmed_diagnostics,
            probable_diagnostics: counts.probable_diagnostics,
            unknown_diagnostics: counts.unknown_diagnostics,
            recommendations: counts.recommendations,
        }
    }
}

/// Counts produced by the core analysis pipeline.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReportCounts {
    /// Data-quality findings.
    pub data_quality_findings: usize,
    /// Confirmed diagnostic findings.
    pub confirmed_diagnostics: usize,
    /// Probable diagnostic findings.
    pub probable_diagnostics: usize,
    /// Unknown diagnostic findings.
    pub unknown_diagnostics: usize,
    /// Recommendations.
    pub recommendations: usize,
}

impl ReportDocument {
    /// Creates a report from already-derived, presentation-neutral records.
    #[must_use]
    pub fn new(
        versions: CoreVersionManifest,
        traces: Vec<TraceReport>,
        alias_count: usize,
    ) -> Self {
        Self {
            versions,
            traces,
            alias_count,
        }
    }
}
