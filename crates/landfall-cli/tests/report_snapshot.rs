//! Deterministic offline report snapshot coverage.
#![allow(clippy::expect_used)]

use landfall_cli::group_traces;
use landfall_core::versions::current_versions;
use landfall_report::{PrivacyProfile, ReportCounts, ReportDocument, TraceReport, render_json};
use std::io::Cursor;

fn report_snapshot() -> String {
    let batch: landfall_protocol::EventBatch = serde_json::from_str(include_str!(
        "../../../fixtures/protocol/v1/valid/incidents/success.batch.json"
    ))
    .expect("fixture");
    let events = batch
        .events
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .expect("serialize")
        .join("\n");
    let parsed = landfall_cli::ingest_ndjson(Cursor::new(events)).expect("ingest");
    let grouped = group_traces(parsed).expect("group");
    let traces = grouped
        .analyses
        .iter()
        .map(|(id, analysis)| {
            TraceReport::from_state(
                *id,
                analysis.projection.trace().state(),
                ReportCounts {
                    data_quality_findings: analysis.data_quality_findings,
                    confirmed_diagnostics: analysis.confirmed_diagnostics,
                    probable_diagnostics: analysis.probable_diagnostics,
                    unknown_diagnostics: analysis.unknown_diagnostics,
                    recommendations: analysis.recommendations.len(),
                },
            )
        })
        .collect();
    render_json(
        &ReportDocument::new(current_versions(), traces, grouped.aliases.len()),
        PrivacyProfile::Internal,
    )
    .expect("json")
}

#[test]
fn repeated_offline_runs_produce_identical_snapshot() {
    assert_eq!(report_snapshot(), report_snapshot());
    let snapshot = report_snapshot();
    assert!(snapshot.contains("\"versions\""));
    assert!(snapshot.contains("\"traces\""));
    assert!(snapshot.contains("\"terminal_eligible\": true"));
}
