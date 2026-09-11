//! Self-contained HTML renderer tests.
#![allow(clippy::expect_used)]

use landfall_core::{
    domain::{
        ApplicationOutcome, ExecutionState, LandingState, LifecycleStage, ObservationCompleteness,
        TraceState,
    },
    versions::current_versions,
};
use landfall_protocol::TraceId;
use landfall_report::{ReportCounts, ReportDocument, TraceReport, render_html};
use std::str::FromStr;

#[test]
fn html_contains_report_data_and_no_external_assets() {
    let trace = TraceId::from_str("0198ef10-0007-7000-8000-000000000201").expect("trace");
    let state = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Finalized,
        execution: ExecutionState::Success,
        application: ApplicationOutcome::Success,
        observation: ObservationCompleteness::Complete,
    };
    let document = ReportDocument::new(
        current_versions(),
        vec![TraceReport::from_state(
            trace,
            state,
            ReportCounts::default(),
        )],
        0,
    );
    let html = render_html(&document);
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("Landfall report"));
    assert!(html.contains("finalized"));
    assert!(!html.contains("<script src=") && !html.contains("https://"));
}
