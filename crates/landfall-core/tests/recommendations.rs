//! Recommendation generation tests.
#![allow(clippy::expect_used)]

use landfall_core::{
    diagnostics::{DiagnosticCertainty, DiagnosticClaimKey, DiagnosticFinding},
    domain::EvidenceSet,
    recommendations::{RecommendationKey, generate_recommendations},
};
use landfall_protocol::EventId;
use std::str::FromStr;

#[test]
fn recommendations_preserve_diagnostic_and_evidence_links() {
    let event = EventId::from_str("0198ef10-0007-7000-8000-000000000200").expect("event");
    let id = landfall_core::domain::DiagnosticId::try_from(event.into_uuid()).expect("id");
    let finding = DiagnosticFinding::test_new(
        id,
        DiagnosticClaimKey::RouteDegradationSignal,
        DiagnosticCertainty::Probable,
        EvidenceSet::new(event),
    );
    let trace = landfall_protocol::TraceId::from_str("0198ef10-0007-7000-8000-000000000201")
        .expect("trace");
    let recommendations = generate_recommendations(trace, &[finding]);
    assert_eq!(recommendations.len(), 1);
    assert_eq!(recommendations[0].key(), RecommendationKey::ReviewRpcRoute);
    assert_eq!(recommendations[0].diagnostic_id(), id);
    assert_eq!(recommendations[0].evidence().len(), 1);
}
