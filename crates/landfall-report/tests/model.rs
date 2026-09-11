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
use landfall_report::{
    ReportCounts, ReportDocument, ReportQueryRow, TraceReport, document_from_query_rows,
    freeze_report_scope,
};
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

#[test]
fn query_rows_build_a_watermarked_snapshot() {
    let trace = TraceId::from_str("0198ef10-0007-7000-8000-000000000201").expect("trace");
    let state = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Finalized,
        execution: ExecutionState::Success,
        application: ApplicationOutcome::Success,
        observation: ObservationCompleteness::Complete,
    };
    let snapshot = document_from_query_rows(
        current_versions(),
        [ReportQueryRow {
            trace_id: trace,
            state,
            counts: ReportCounts::default(),
        }],
        0,
        184_220_941,
    );
    assert_eq!(snapshot.projection_watermark, 184_220_941);
    assert_eq!(snapshot.document.traces.len(), 1);
}

#[test]
fn report_scope_freezes_versions_and_rejects_empty_identity() {
    let scope =
        freeze_report_scope("cohort-a", 17, "events-v1", current_versions()).expect("scope");
    assert_eq!(scope.projection_watermark, 17);
    assert_eq!(scope.schema_version, "events-v1");
    assert!(freeze_report_scope("", 17, "events-v1", current_versions()).is_err());
}
