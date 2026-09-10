//! Deterministic full-replay tests for the pure trace reducer.

use std::str::FromStr;

use landfall_core::{
    domain::{
        ApplicationOutcome, ExecutionState, LandingState, LifecycleStage, ObservationCompleteness,
        TraceState,
    },
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
    reducer::{REDUCER_VERSION, ReducerError, ReducerWarning, reduce_trace},
};
use landfall_protocol::{EventBatch, TraceId, UtcTimestamp, WireEvent};

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::error::Error + 'static,
{
    Ok(value.parse()?)
}

fn fixture(path: &str) -> Result<EventBatch, Box<dyn std::error::Error>> {
    let json = match path {
        "success" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/success.batch.json")
        }
        "timeout-later-success" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/timeout-later-success.batch.json"
        ),
        "expiry-without-inclusion" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/expiry-without-inclusion.batch.json"
        ),
        "observer-disagreement" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/observer-disagreement.batch.json"
        ),
        "unsupported-transaction-features" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/unsupported-transaction-features.batch.json"
        ),
        _ => return Err(std::io::Error::other("unknown reducer fixture").into()),
    };
    Ok(serde_json::from_str(json)?)
}

fn order_batch(
    batch: EventBatch,
) -> Result<landfall_core::ordering::CanonicalOrder, Box<dyn std::error::Error>> {
    let events = batch
        .events
        .into_iter()
        .map(|event| CollectedEvent::new(event, batch.sent_at));
    Ok(canonical_order(events, OrderingConfig::default())?)
}

fn order_events(
    events: impl IntoIterator<Item = WireEvent>,
) -> Result<landfall_core::ordering::CanonicalOrder, Box<dyn std::error::Error>> {
    let received_at = parse::<UtcTimestamp>("2026-08-29T12:01:00Z")?;
    Ok(canonical_order(
        events
            .into_iter()
            .map(|event| CollectedEvent::new(event, received_at)),
        OrderingConfig::default(),
    )?)
}

#[test]
fn successful_fixture_reduces_all_independent_dimensions() -> Result<(), Box<dyn std::error::Error>>
{
    let projection = reduce_trace(&order_batch(fixture("success")?)?)?;

    assert_eq!(projection.reducer_version(), REDUCER_VERSION);
    assert_eq!(
        projection.trace().state(),
        TraceState {
            lifecycle: LifecycleStage::Observed,
            landing: LandingState::Finalized,
            execution: ExecutionState::Success,
            application: ApplicationOutcome::Success,
            observation: ObservationCompleteness::Complete,
        }
    );
    assert_eq!(projection.trace().evidence().len(), 2);
    assert_eq!(projection.warnings(), [ReducerWarning::MissingTraceCreated]);
    Ok(())
}

#[test]
fn transport_timeout_does_not_override_later_network_success()
-> Result<(), Box<dyn std::error::Error>> {
    let projection = reduce_trace(&order_batch(fixture("timeout-later-success")?)?)?;
    let state = projection.trace().state();

    assert_eq!(state.lifecycle, LifecycleStage::Observed);
    assert_eq!(state.landing, LandingState::Finalized);
    assert_eq!(state.execution, ExecutionState::Success);
    assert_eq!(state.application, ApplicationOutcome::Unknown);
    assert_eq!(state.observation, ObservationCompleteness::Complete);
    Ok(())
}

#[test]
fn block_height_past_validity_window_derives_expiration() -> Result<(), Box<dyn std::error::Error>>
{
    let projection = reduce_trace(&order_batch(fixture("expiry-without-inclusion")?)?)?;
    let state = projection.trace().state();

    assert_eq!(state.lifecycle, LifecycleStage::Observed);
    assert_eq!(state.landing, LandingState::Expired);
    assert_eq!(state.execution, ExecutionState::Unknown);
    assert_eq!(state.observation, ObservationCompleteness::Complete);
    Ok(())
}

#[test]
fn explicit_observer_disagreement_is_retained_as_conflicting()
-> Result<(), Box<dyn std::error::Error>> {
    let projection = reduce_trace(&order_batch(fixture("observer-disagreement")?)?)?;

    assert_eq!(
        projection.trace().state().landing,
        LandingState::Conflicting
    );
    assert!(
        projection
            .warnings()
            .contains(&ReducerWarning::ObserverDisagreement)
    );
    Ok(())
}

#[test]
fn observation_incomplete_quality_evidence_is_not_called_expired()
-> Result<(), Box<dyn std::error::Error>> {
    let mut batch = fixture("unsupported-transaction-features")?;
    batch.events.truncate(2);
    let projection = reduce_trace(&order_batch(batch)?)?;
    let state = projection.trace().state();

    assert_eq!(state.landing, LandingState::Incomplete);
    assert_eq!(state.observation, ObservationCompleteness::Incomplete);
    assert!(!state.observation.is_metric_eligible());
    Ok(())
}

#[test]
fn permutation_and_duplicate_delivery_replay_to_the_same_projection()
-> Result<(), Box<dyn std::error::Error>> {
    let batch = fixture("timeout-later-success")?;
    let forward = reduce_trace(&order_events(batch.events.clone())?)?;
    let mut reordered = batch.events;
    reordered.push(reordered[1].clone());
    reordered.reverse();
    let replayed = reduce_trace(&order_events(reordered)?)?;

    assert_eq!(forward, replayed);
    Ok(())
}

#[test]
fn mixed_trace_evidence_is_rejected_before_projection() -> Result<(), Box<dyn std::error::Error>> {
    let mut batch = fixture("success")?;
    let foreign_trace = parse::<TraceId>("0198ef10-0001-7000-8000-000000000101")?;
    let conflicting_event_id;
    if let WireEvent::BusinessOutcomeObserved(event) = &mut batch.events[1] {
        event.trace_id = Some(foreign_trace);
        conflicting_event_id = event.event_id;
    } else {
        return Err(std::io::Error::other("unexpected success fixture event").into());
    }

    assert_eq!(
        reduce_trace(&order_batch(batch)?),
        Err(ReducerError::TraceMismatch {
            event_id: conflicting_event_id,
        })
    );
    Ok(())
}

#[test]
fn empty_event_set_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let ordered = order_events([])?;
    assert_eq!(reduce_trace(&ordered), Err(ReducerError::EmptyInput));
    Ok(())
}
