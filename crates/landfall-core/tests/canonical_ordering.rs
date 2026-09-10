//! Canonical event ordering, deduplication, and clock-quality warnings.

use std::{str::FromStr, time::Duration};

use landfall_core::ordering::{
    CollectedEvent, OrderingConfig, OrderingError, OrderingWarning, canonical_order,
};
use landfall_protocol::{DurationNs, EventBatch, SourceInstanceId, UtcTimestamp, WireEvent};

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::error::Error + 'static,
{
    Ok(value.parse()?)
}

fn timeout_fixture() -> Result<EventBatch, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(include_str!(
        "../../../fixtures/protocol/v1/valid/incidents/timeout-later-success.batch.json"
    ))?)
}

fn all_events_fixture() -> Result<EventBatch, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(include_str!(
        "../../../fixtures/protocol/v1/valid/all-event-types.batch.json"
    ))?)
}

fn collected(
    events: impl IntoIterator<Item = WireEvent>,
    received_at: UtcTimestamp,
) -> Vec<CollectedEvent> {
    events
        .into_iter()
        .map(|event| CollectedEvent::new(event, received_at))
        .collect()
}

#[test]
fn permutations_produce_the_same_total_order() -> Result<(), Box<dyn std::error::Error>> {
    let batch = timeout_fixture()?;
    let received_at = parse("2026-08-29T12:00:03Z")?;
    let forward = collected(batch.events.clone(), received_at);
    let mut reverse = forward.clone();
    reverse.reverse();

    let forward = canonical_order(forward, OrderingConfig::default())?;
    let reverse = canonical_order(reverse, OrderingConfig::default())?;
    let ids = |ordered: &landfall_core::ordering::CanonicalOrder| {
        ordered
            .events()
            .iter()
            .map(CollectedEvent::event_id)
            .collect::<Vec<_>>()
    };

    assert_eq!(ids(&forward), ids(&reverse));
    assert_eq!(forward.warnings(), reverse.warnings());
    Ok(())
}

#[test]
fn semantic_span_order_wins_over_monotonic_and_wall_clocks()
-> Result<(), Box<dyn std::error::Error>> {
    let batch = all_events_fixture()?;
    let mut started = batch
        .events
        .iter()
        .find(|event| matches!(event, WireEvent::SubmissionStarted(_)))
        .cloned()
        .ok_or_else(|| std::io::Error::other("submission start fixture is missing"))?;
    let mut completed = batch
        .events
        .iter()
        .find(|event| matches!(event, WireEvent::SubmissionCompleted(_)))
        .cloned()
        .ok_or_else(|| std::io::Error::other("submission completion fixture is missing"))?;
    let instance_id = parse::<SourceInstanceId>("0198ef00-0000-7000-8000-000000000600")?;
    let start_id;
    let completed_id;
    if let WireEvent::SubmissionStarted(event) = &mut started {
        event.occurred_at = parse("2026-08-29T12:00:02Z")?;
        event.monotonic_ns = Some(DurationNs::new(200));
        event.source.instance_id = Some(instance_id);
        start_id = event.event_id;
    } else {
        return Err(std::io::Error::other("wrong start event variant").into());
    }
    if let WireEvent::SubmissionCompleted(event) = &mut completed {
        event.occurred_at = parse("2026-08-29T12:00:01Z")?;
        event.monotonic_ns = Some(DurationNs::new(100));
        event.source.instance_id = Some(instance_id);
        completed_id = event.event_id;
    } else {
        return Err(std::io::Error::other("wrong completion event variant").into());
    }

    let ordered = canonical_order(
        collected([completed, started], parse("2026-08-29T12:00:03Z")?),
        OrderingConfig::default(),
    )?;
    assert_eq!(
        ordered
            .events()
            .iter()
            .map(CollectedEvent::event_id)
            .collect::<Vec<_>>(),
        [start_id, completed_id]
    );
    assert!(
        ordered
            .warnings()
            .contains(&OrderingWarning::MonotonicSemanticConflict {
                monotonic_earlier_event_id: completed_id,
                semantic_earlier_event_id: start_id,
            })
    );
    Ok(())
}

#[test]
fn clock_skew_and_wall_clock_regression_are_reported() -> Result<(), Box<dyn std::error::Error>> {
    let batch = all_events_fixture()?;
    let mut started = batch
        .events
        .iter()
        .find(|event| matches!(event, WireEvent::SigningStarted(_)))
        .cloned()
        .ok_or_else(|| std::io::Error::other("signing start fixture is missing"))?;
    let mut completed = batch
        .events
        .iter()
        .find(|event| matches!(event, WireEvent::SigningCompleted(_)))
        .cloned()
        .ok_or_else(|| std::io::Error::other("signing completion fixture is missing"))?;
    let instance_id = parse::<SourceInstanceId>("0198ef00-0000-7000-8000-000000000600")?;
    let start_id;
    let completed_id;
    if let WireEvent::SigningStarted(event) = &mut started {
        event.occurred_at = parse("2026-08-29T12:10:00Z")?;
        event.monotonic_ns = Some(DurationNs::new(100));
        event.source.instance_id = Some(instance_id);
        start_id = event.event_id;
    } else {
        return Err(std::io::Error::other("wrong start event variant").into());
    }
    if let WireEvent::SigningCompleted(event) = &mut completed {
        event.occurred_at = parse("2026-08-29T12:00:00Z")?;
        event.monotonic_ns = Some(DurationNs::new(200));
        event.source.instance_id = Some(instance_id);
        completed_id = event.event_id;
    } else {
        return Err(std::io::Error::other("wrong completion event variant").into());
    }

    let ordered = canonical_order(
        collected([completed, started], parse("2026-08-29T13:00:00Z")?),
        OrderingConfig {
            max_clock_skew: Duration::from_secs(60),
        },
    )?;
    assert!(ordered.warnings().iter().any(|warning| matches!(
        warning,
        OrderingWarning::ClockSkew { event_id, .. } if *event_id == start_id
    )));
    assert!(ordered.warnings().iter().any(|warning| matches!(
        warning,
        OrderingWarning::WallClockRegression {
            earlier_event_id,
            later_event_id,
            ..
        } if *earlier_event_id == start_id && *later_event_id == completed_id
    )));
    Ok(())
}

#[test]
fn duplicates_collapse_but_conflicting_reuse_is_rejected() -> Result<(), Box<dyn std::error::Error>>
{
    let batch = timeout_fixture()?;
    let first = batch
        .events
        .first()
        .cloned()
        .ok_or_else(|| std::io::Error::other("timeout fixture is empty"))?;
    let first_id = match &first {
        WireEvent::SubmissionCompleted(event) => event.event_id,
        _ => return Err(std::io::Error::other("unexpected first fixture event").into()),
    };
    let received_at = parse("2026-08-29T12:00:03Z")?;
    let ordered = canonical_order(
        collected([first.clone(), first.clone()], received_at),
        OrderingConfig::default(),
    )?;
    assert_eq!(ordered.events().len(), 1);
    assert!(
        ordered
            .warnings()
            .contains(&OrderingWarning::DuplicateDelivery {
                event_id: first_id,
                copies: 2,
            })
    );

    let mut conflicting = batch
        .events
        .get(1)
        .cloned()
        .ok_or_else(|| std::io::Error::other("second timeout fixture event is missing"))?;
    if let WireEvent::StatusObserved(event) = &mut conflicting {
        event.event_id = first_id;
    } else {
        return Err(std::io::Error::other("unexpected second fixture event").into());
    }
    assert_eq!(
        canonical_order(
            collected([first, conflicting], received_at),
            OrderingConfig::default(),
        ),
        Err(OrderingError::ConflictingDuplicate { event_id: first_id })
    );
    Ok(())
}

#[test]
fn unrelated_source_clocks_are_marked_incomparable() -> Result<(), Box<dyn std::error::Error>> {
    let batch = timeout_fixture()?;
    let ordered = canonical_order(
        collected(batch.events, parse("2026-08-29T12:00:03Z")?),
        OrderingConfig::default(),
    )?;

    assert!(
        ordered
            .warnings()
            .iter()
            .any(|warning| matches!(warning, OrderingWarning::IncomparableClockOrder { .. }))
    );
    Ok(())
}

#[test]
fn equal_monotonic_positions_are_not_treated_as_ordered() -> Result<(), Box<dyn std::error::Error>>
{
    let mut events = timeout_fixture()?.events;
    events.truncate(2);
    let instance_id = parse::<SourceInstanceId>("0198ef00-0000-7000-8000-000000000600")?;
    for event in &mut events {
        match event {
            WireEvent::SubmissionCompleted(event) => {
                event.monotonic_ns = Some(DurationNs::new(100));
                event.source.instance_id = Some(instance_id);
            }
            WireEvent::StatusObserved(event) => {
                event.monotonic_ns = Some(DurationNs::new(100));
                event.source.instance_id = Some(instance_id);
            }
            _ => return Err(std::io::Error::other("unexpected timeout fixture event").into()),
        }
    }
    let ordered = canonical_order(
        collected(events, parse("2026-08-29T12:00:03Z")?),
        OrderingConfig::default(),
    )?;

    assert!(
        ordered
            .warnings()
            .iter()
            .any(|warning| matches!(warning, OrderingWarning::IncomparableClockOrder { .. }))
    );
    Ok(())
}
