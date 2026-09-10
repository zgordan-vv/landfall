//! Golden tests for probable diagnostic risk signals.
#![allow(clippy::expect_used, clippy::panic)]

use std::str::FromStr;

use landfall_core::{
    diagnostics::{
        DiagnosticCertainty, DiagnosticClaimKey, ProbableDiagnosticConfig,
        evaluate_probable_diagnostics,
    },
    grouping::group_trace,
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
    reducer::reduce_trace,
};
use landfall_protocol::{EventBatch, UtcTimestamp, WireEvent};

fn batch(name: &str) -> EventBatch {
    let json = match name {
        "success" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/success.batch.json")
        }
        "retry" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/identical-retry.batch.json")
        }
        _ => panic!("unknown fixture"),
    };
    serde_json::from_str(json).expect("valid fixture")
}

fn evaluate(
    mut events: Vec<WireEvent>,
    config: ProbableDiagnosticConfig,
) -> Vec<landfall_core::diagnostics::DiagnosticFinding> {
    let received_at = UtcTimestamp::from_str("2026-08-29T12:01:00Z").expect("timestamp");
    let ordered = canonical_order(
        events
            .drain(..)
            .map(|event| CollectedEvent::new(event, received_at)),
        OrderingConfig::default(),
    )
    .expect("order");
    let projection = reduce_trace(&ordered).expect("projection");
    let grouping = group_trace(&ordered).expect("grouping");
    evaluate_probable_diagnostics(&ordered, &projection, &grouping, config)
}

#[test]
fn excessive_signing_delay_is_probable() {
    let mut events = batch("retry").events;
    if let WireEvent::SigningCompleted(event) = &mut events[0] {
        event.attributes.duration_ns = "30000000000".parse().expect("duration");
    }
    let findings = evaluate(events, ProbableDiagnosticConfig::default());
    assert!(findings.iter().any(|finding| finding.claim_key()
        == DiagnosticClaimKey::ExcessiveSigningDelay
        && finding.certainty() == DiagnosticCertainty::Probable));
}

#[test]
fn retries_and_route_failures_emit_probable_signals() {
    let findings = evaluate(
        batch("retry").events,
        ProbableDiagnosticConfig {
            route_failure_count_threshold: 1,
            ..Default::default()
        },
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.claim_key() == DiagnosticClaimKey::RouteDegradationSignal)
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.claim_key() == DiagnosticClaimKey::UnsafeRedundantRetry)
    );
}

#[test]
fn fee_rule_waits_for_fee_market_evidence() {
    let findings = evaluate(batch("success").events, ProbableDiagnosticConfig::default());
    assert!(
        !findings
            .iter()
            .any(|finding| finding.claim_key() == DiagnosticClaimKey::FeeLikelyUncompetitive)
    );
}
