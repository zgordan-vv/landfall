//! Golden tests for explicit unknown and missing-evidence findings.
#![allow(clippy::expect_used, clippy::panic)]

use std::str::FromStr;

use landfall_core::{
    diagnostics::{DiagnosticCertainty, DiagnosticClaimKey, evaluate_unknown_diagnostics},
    grouping::group_trace,
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
    reducer::reduce_trace,
};
use landfall_protocol::{EventBatch, UtcTimestamp};

fn evaluate(name: &str) -> Vec<landfall_core::diagnostics::DiagnosticFinding> {
    let json = match name {
        "success" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/success.batch.json")
        }
        _ => panic!("unknown fixture"),
    };
    let batch: EventBatch = serde_json::from_str(json).expect("valid fixture");
    let received_at = UtcTimestamp::from_str("2026-08-29T12:01:00Z").expect("timestamp");
    let ordered = canonical_order(
        batch
            .events
            .into_iter()
            .map(|event| CollectedEvent::new(event, received_at)),
        OrderingConfig::default(),
    )
    .expect("order");
    let projection = reduce_trace(&ordered).expect("projection");
    let grouping = group_trace(&ordered).expect("grouping");
    evaluate_unknown_diagnostics(&ordered, &projection, &grouping)
}

#[test]
fn missing_evidence_is_explicit_and_unknown() {
    let findings = evaluate("success");
    assert!(!findings.is_empty());
    assert!(findings.iter().all(|finding| finding.claim_key()
        == DiagnosticClaimKey::MissingEvidence
        && finding.certainty() == DiagnosticCertainty::Unknown));
    assert!(
        findings
            .iter()
            .all(|finding| finding.unknown_reason().is_some())
    );
}
