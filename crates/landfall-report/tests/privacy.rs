//! Privacy-profile export tests.
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
    PrivacyProfile, ReportCounts, ReportDocument, TraceReport, render_html_with_profile,
    render_json,
};
use std::str::FromStr;

#[test]
fn shareable_export_redacts_trace_identity() {
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
    let json = render_json(&document, PrivacyProfile::Shareable).expect("json");
    let html = render_html_with_profile(&document, PrivacyProfile::Shareable);
    assert!(json.contains("[redacted]") && !json.contains(&trace.to_string()));
    assert!(html.contains("[redacted]") && !html.contains(&trace.to_string()));
}
