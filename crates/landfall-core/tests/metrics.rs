//! Metric-definition tests.
#![allow(clippy::expect_used)]

use landfall_core::{
    metrics::{
        METRIC_DEFINITIONS_VERSION, landed_numerator, terminal_denominator_eligible,
        trace_metric_flags,
    },
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
    reducer::reduce_trace,
};
use landfall_protocol::{EventBatch, UtcTimestamp};

#[test]
fn successful_trace_enters_terminal_denominator_and_landing_numerator() {
    let batch: EventBatch = serde_json::from_str(include_str!(
        "../../../fixtures/protocol/v1/valid/incidents/success.batch.json"
    ))
    .expect("fixture");
    let received = "2026-08-29T12:01:00Z"
        .parse::<UtcTimestamp>()
        .expect("timestamp");
    let ordered = canonical_order(
        batch
            .events
            .into_iter()
            .map(|event| CollectedEvent::new(event, received)),
        OrderingConfig::default(),
    )
    .expect("order");
    let projection = reduce_trace(&ordered).expect("projection");
    let flags = trace_metric_flags(&projection);
    assert_eq!(METRIC_DEFINITIONS_VERSION, "metric-definitions-v1");
    assert!(flags.terminal_eligible);
    assert!(flags.landed);
    assert!(flags.execution_success);
    assert!(flags.application_success);
    assert!(terminal_denominator_eligible(&projection));
    assert!(landed_numerator(&projection));
}
