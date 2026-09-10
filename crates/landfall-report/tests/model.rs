//! Report-domain model tests.
#![allow(clippy::expect_used)]

use landfall_core::{
    domain::{
        ApplicationOutcome, ExecutionState, LandingState, LifecycleStage, ObservationCompleteness,
        TraceState,
    },
    versions::current_versions,
};
use landfall_protocol::TraceId;
use landfall_report::{ReportCounts, ReportDocument, TraceReport};
use std::str::FromStr;

#[test]
fn state_is_projected_to_stable_report_tokens() {
    let trace = TraceId::from_str("0198ef10-0007-7000-8000-000000000201").expect("trace");
    let state = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Finalized,
        execution: ExecutionState::Success,
        application: ApplicationOutcome::Success,
        observation: ObservationCompleteness::Complete,
    };
    let report = TraceReport::from_state(trace, state, ReportCounts::default());
    assert_eq!(report.landing, "finalized");
    assert!(report.terminal_eligible);
    let document = ReportDocument::new(current_versions(), vec![report], 0);
    assert_eq!(document.traces.len(), 1);
}
