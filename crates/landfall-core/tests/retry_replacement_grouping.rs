//! Golden tests for retry, replacement, and alias-candidate classification.

use std::{collections::BTreeMap, str::FromStr};

use landfall_core::{
    grouping::{
        GroupingWarning, TraceGrouping, TraceRelationship, classify_trace_relationship, group_trace,
    },
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
};
use landfall_protocol::{
    BusinessActionId, EventBatch, FingerprintHex, TraceId, UtcTimestamp, WireEvent,
};

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::error::Error + 'static,
{
    Ok(value.parse()?)
}

fn fixture(name: &str) -> Result<EventBatch, Box<dyn std::error::Error>> {
    let json = match name {
        "identical-retry" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/identical-retry.batch.json")
        }
        "replacement" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/replacement-transaction.batch.json"
        ),
        "two-successful-replacements" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/two-successful-replacements.batch.json"
        ),
        "all-events" => {
            include_str!("../../../fixtures/protocol/v1/valid/all-event-types.batch.json")
        }
        _ => return Err(std::io::Error::other("unknown grouping fixture").into()),
    };
    Ok(serde_json::from_str(json)?)
}

fn event_trace_id(event: &WireEvent) -> Option<TraceId> {
    macro_rules! trace_match {
        ($event:expr, $binding:ident => $result:expr) => {
            match $event {
                WireEvent::TraceCreated($binding) => $result,
                WireEvent::BlockhashAcquired($binding) => $result,
                WireEvent::SimulationStarted($binding) => $result,
                WireEvent::SimulationCompleted($binding) => $result,
                WireEvent::SigningStarted($binding) => $result,
                WireEvent::SigningCompleted($binding) => $result,
                WireEvent::SubmissionStarted($binding) => $result,
                WireEvent::SubmissionCompleted($binding) => $result,
                WireEvent::SubmissionRetryScheduled($binding) => $result,
                WireEvent::ConfirmationWaitStarted($binding) => $result,
                WireEvent::ConfirmationWaitCompleted($binding) => $result,
                WireEvent::StatusObserved($binding) => $result,
                WireEvent::ExecutionEnriched($binding) => $result,
                WireEvent::BusinessOutcomeObserved($binding) => $result,
                WireEvent::DataQualityDetected($binding) => $result,
            }
        };
    }
    trace_match!(event, value => value.trace_id)
}

fn order(
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

fn groups(batch: EventBatch) -> Result<Vec<TraceGrouping>, Box<dyn std::error::Error>> {
    let mut by_trace = BTreeMap::<TraceId, Vec<WireEvent>>::new();
    for event in batch.events {
        let trace_id = event_trace_id(&event)
            .ok_or_else(|| std::io::Error::other("fixture event has no trace"))?;
        by_trace.entry(trace_id).or_default().push(event);
    }
    by_trace
        .into_values()
        .map(|events| Ok(group_trace(&order(events)?)?))
        .collect()
}

#[test]
fn distinct_attempt_ids_inside_one_trace_are_retries() -> Result<(), Box<dyn std::error::Error>> {
    let grouping = group_trace(&order(fixture("identical-retry")?.events)?)?;

    assert_eq!(grouping.attempts().len(), 2);
    assert!(grouping.has_retries());
    assert_eq!(grouping.scheduled_retry_count(), 0);
    assert!(grouping.attempts().iter().all(|attempt| {
        attempt.evidence().started_event_id().is_some()
            && attempt.evidence().completed_event_id().is_some()
    }));
    assert_ne!(grouping.attempts()[0].id(), grouping.attempts()[1].id());
    Ok(())
}

#[test]
fn attempt_sequence_is_display_metadata_not_identity() -> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("identical-retry")?.events;
    if let WireEvent::SubmissionStarted(event) = &mut events[3] {
        event.attributes.attempt_sequence = landfall_protocol::AttemptSequence::try_from(1)?;
    } else {
        return Err(std::io::Error::other("expected second attempt start").into());
    }

    let grouping = group_trace(&order(events)?)?;
    assert_eq!(grouping.attempts().len(), 2);
    assert!(grouping.has_retries());
    Ok(())
}

#[test]
fn scheduling_a_retry_does_not_invent_an_attempt() -> Result<(), Box<dyn std::error::Error>> {
    let retry = fixture("all-events")?
        .events
        .into_iter()
        .find(|event| matches!(event, WireEvent::SubmissionRetryScheduled(_)))
        .ok_or_else(|| std::io::Error::other("retry fixture event is missing"))?;
    let grouping = group_trace(&order([retry])?)?;

    assert!(grouping.attempts().is_empty());
    assert_eq!(grouping.scheduled_retry_count(), 1);
    assert!(matches!(
        grouping.warnings(),
        [GroupingWarning::RetryReferencesMissingAttempt { .. }]
    ));
    Ok(())
}

#[test]
fn distinct_traces_under_one_action_are_replacements() -> Result<(), Box<dyn std::error::Error>> {
    let grouped = groups(fixture("replacement")?)?;
    assert_eq!(grouped.len(), 2);
    assert_eq!(
        classify_trace_relationship(&grouped[0], &grouped[1])?,
        TraceRelationship::Replacement
    );
    Ok(())
}

#[test]
fn replacement_classification_does_not_require_private_identity_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let grouped = groups(fixture("two-successful-replacements")?)?;
    assert!(grouped.iter().all(|group| group.signatures().is_empty()));
    assert!(grouped.iter().all(|group| group.fingerprints().is_empty()));
    assert_eq!(
        classify_trace_relationship(&grouped[0], &grouped[1])?,
        TraceRelationship::Replacement
    );
    Ok(())
}

#[test]
fn matching_fingerprint_across_trace_ids_creates_an_alias_candidate()
-> Result<(), Box<dyn std::error::Error>> {
    let mut batch = fixture("replacement")?;
    if let WireEvent::SigningCompleted(event) = &mut batch.events[3] {
        event
            .attributes
            .signed_bytes_fingerprint
            .as_mut()
            .ok_or_else(|| std::io::Error::other("replacement fingerprint is missing"))?
            .value_hex = parse::<FingerprintHex>(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )?;
    } else {
        return Err(std::io::Error::other("expected second signing completion").into());
    }
    let grouped = groups(batch)?;

    assert_eq!(
        classify_trace_relationship(&grouped[0], &grouped[1])?,
        TraceRelationship::SameSignedTransactionAliasCandidate
    );
    Ok(())
}

#[test]
fn incompatible_fingerprints_inside_one_trace_remain_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let mut batch = fixture("replacement")?;
    let first_trace = event_trace_id(&batch.events[0])
        .ok_or_else(|| std::io::Error::other("first trace is missing"))?;
    batch.events.remove(2);
    if let WireEvent::SigningCompleted(event) = &mut batch.events[2] {
        event.trace_id = Some(first_trace);
    } else {
        return Err(std::io::Error::other("expected second signing completion").into());
    }
    let grouping = group_trace(&order(batch.events)?)?;

    assert!(
        grouping
            .warnings()
            .contains(&GroupingWarning::ConflictingSignedIdentity)
    );
    assert_eq!(
        classify_trace_relationship(&grouping, &grouping)?,
        TraceRelationship::ConflictingIdentityEvidence
    );
    Ok(())
}

#[test]
fn same_signed_identity_with_different_actions_is_a_conflict()
-> Result<(), Box<dyn std::error::Error>> {
    let mut batch = fixture("replacement")?;
    let other_action = parse::<BusinessActionId>("0198ef10-0008-7000-8000-000000000201")?;
    for event in &mut batch.events[2..] {
        match event {
            WireEvent::TraceCreated(event) => event.business_action_id = Some(other_action),
            WireEvent::SigningCompleted(event) => {
                event.business_action_id = Some(other_action);
                event
                    .attributes
                    .signed_bytes_fingerprint
                    .as_mut()
                    .ok_or_else(|| std::io::Error::other("fingerprint is missing"))?
                    .value_hex = parse::<FingerprintHex>(
                    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                )?;
            }
            _ => return Err(std::io::Error::other("unexpected replacement event").into()),
        }
    }
    let grouped = groups(batch)?;

    assert_eq!(
        classify_trace_relationship(&grouped[0], &grouped[1])?,
        TraceRelationship::ConflictingIdentityEvidence
    );
    Ok(())
}

#[test]
fn absent_business_action_correlation_is_reported_not_guessed()
-> Result<(), Box<dyn std::error::Error>> {
    let mut batch = fixture("replacement")?;
    for event in &mut batch.events {
        match event {
            WireEvent::TraceCreated(event) => event.business_action_id = None,
            WireEvent::SigningCompleted(event) => event.business_action_id = None,
            _ => return Err(std::io::Error::other("unexpected replacement event").into()),
        }
    }
    let grouped = groups(batch)?;

    assert_eq!(
        classify_trace_relationship(&grouped[0], &grouped[1])?,
        TraceRelationship::GroupingUnavailable
    );
    Ok(())
}

#[test]
fn input_permutations_produce_the_same_attempt_group() -> Result<(), Box<dyn std::error::Error>> {
    let events = fixture("identical-retry")?.events;
    let forward = group_trace(&order(events.clone())?)?;
    let mut reverse = events;
    reverse.reverse();
    let reversed = group_trace(&order(reverse)?)?;

    assert_eq!(forward, reversed);
    Ok(())
}
