//! Golden and contextual tests for the versioned data-quality rubric.

use std::str::FromStr;

use landfall_core::{
    data_quality::{
        DATA_QUALITY_VERSION, DataQualityError, DataQualityFindingCode, DataQualityGrade,
        evaluate_data_quality,
    },
    domain::ExecutionState,
    grouping::group_trace,
    ordering::{CanonicalOrder, CollectedEvent, OrderingConfig, canonical_order},
    reducer::{TraceProjection, reduce_trace},
};
use landfall_protocol::{
    DataQualityCategory, DataQualitySeverity, EventBatch, PrivacyMode, UtcTimestamp, WireEvent,
};

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::error::Error + 'static,
{
    Ok(value.parse()?)
}

fn fixture(name: &str) -> Result<EventBatch, Box<dyn std::error::Error>> {
    let json = match name {
        "all-events" => {
            include_str!("../../../fixtures/protocol/v1/valid/all-event-types.batch.json")
        }
        "success" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/success.batch.json")
        }
        "missing-block-height" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/missing-block-height.batch.json"
        ),
        "identical-retry" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/identical-retry.batch.json")
        }
        "unsupported-transaction-features" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/unsupported-transaction-features.batch.json"
        ),
        _ => return Err(std::io::Error::other("unknown data-quality fixture").into()),
    };
    Ok(serde_json::from_str(json)?)
}

fn order(
    events: impl IntoIterator<Item = WireEvent>,
) -> Result<CanonicalOrder, Box<dyn std::error::Error>> {
    let received_at = parse::<UtcTimestamp>("2026-08-29T12:01:00Z")?;
    Ok(canonical_order(
        events
            .into_iter()
            .map(|event| CollectedEvent::new(event, received_at)),
        OrderingConfig::default(),
    )?)
}

fn project(events: &CanonicalOrder) -> Result<TraceProjection, Box<dyn std::error::Error>> {
    Ok(reduce_trace(events)?)
}

fn has_code(
    assessment: &landfall_core::data_quality::DataQualityAssessment,
    code: DataQualityFindingCode,
) -> bool {
    assessment
        .findings()
        .iter()
        .any(|finding| finding.code() == code)
}

#[test]
fn complete_fixture_uses_the_documented_version_and_stable_band()
-> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("all-events")?.events;
    let trace_id = match &events[0] {
        WireEvent::TraceCreated(event) => event.trace_id,
        _ => return Err(std::io::Error::other("trace fixture is missing").into()),
    };
    for event in &mut events {
        match event {
            WireEvent::BusinessOutcomeObserved(event) => event.trace_id = trace_id,
            WireEvent::DataQualityDetected(event) => event.trace_id = trace_id,
            _ => {}
        }
    }
    let ordered = order(events)?;
    let projection = project(&ordered)?;
    let grouping = group_trace(&ordered)?;
    let assessment = evaluate_data_quality(&ordered, &projection, &grouping)?;

    assert_eq!(assessment.version(), DATA_QUALITY_VERSION);
    assert_eq!(assessment.grade(), DataQualityGrade::B);
    assert_eq!(assessment.grade().as_char(), 'B');
    assert!(has_code(
        &assessment,
        DataQualityFindingCode::Reported(DataQualityCategory::ObserverGap)
    ));
    Ok(())
}

#[test]
fn transaction_success_does_not_imply_high_data_quality() -> Result<(), Box<dyn std::error::Error>>
{
    let ordered = order(fixture("success")?.events)?;
    let projection = project(&ordered)?;
    let grouping = group_trace(&ordered)?;
    let assessment = evaluate_data_quality(&ordered, &projection, &grouping)?;

    assert_eq!(
        projection.trace().state().execution,
        ExecutionState::Success
    );
    assert!(assessment.grade() < DataQualityGrade::B);
    assert!(has_code(
        &assessment,
        DataQualityFindingCode::MissingSigningEvidence
    ));
    assert!(has_code(
        &assessment,
        DataQualityFindingCode::MissingSubmissionEvidence
    ));
    Ok(())
}

#[test]
fn explicit_missing_block_height_is_retained_with_its_evidence()
-> Result<(), Box<dyn std::error::Error>> {
    let ordered = order(fixture("missing-block-height")?.events)?;
    let projection = project(&ordered)?;
    let grouping = group_trace(&ordered)?;
    let assessment = evaluate_data_quality(&ordered, &projection, &grouping)?;
    let finding = assessment
        .findings()
        .iter()
        .find(|finding| {
            finding.code()
                == DataQualityFindingCode::Reported(DataQualityCategory::BlockHeightMissing)
        })
        .ok_or_else(|| std::io::Error::other("reported block-height gap is missing"))?;

    assert_eq!(finding.severity(), DataQualitySeverity::Warning);
    assert_eq!(finding.evidence().len(), 1);
    assert!(has_code(
        &assessment,
        DataQualityFindingCode::MissingLastValidBlockHeight
    ));
    Ok(())
}

#[test]
fn missing_attempt_completion_is_listed_individually() -> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("identical-retry")?.events;
    events.truncate(2);
    let attempt_id = match &events[1] {
        WireEvent::SubmissionStarted(event) => event.attributes.attempt_id,
        _ => return Err(std::io::Error::other("attempt start fixture is missing").into()),
    };
    let ordered = order(events)?;
    let projection = project(&ordered)?;
    let grouping = group_trace(&ordered)?;
    let assessment = evaluate_data_quality(&ordered, &projection, &grouping)?;

    assert!(has_code(
        &assessment,
        DataQualityFindingCode::MissingSubmissionResponse { attempt_id }
    ));
    Ok(())
}

#[test]
fn strict_privacy_omission_is_distinguished_from_telemetry_loss()
-> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("identical-retry")?.events;
    for event in &mut events {
        match event {
            WireEvent::SigningCompleted(event) => {
                event.privacy_mode = PrivacyMode::Strict;
                event.attributes.signed_bytes_fingerprint = None;
                event.attributes.signature = None;
            }
            WireEvent::SubmissionStarted(event) => event.privacy_mode = PrivacyMode::Strict,
            WireEvent::SubmissionCompleted(event) => {
                event.privacy_mode = PrivacyMode::Strict;
                event.attributes.signature = None;
            }
            _ => return Err(std::io::Error::other("unexpected retry fixture event").into()),
        }
    }
    let ordered = order(events)?;
    let projection = project(&ordered)?;
    let grouping = group_trace(&ordered)?;
    let assessment = evaluate_data_quality(&ordered, &projection, &grouping)?;

    assert!(has_code(
        &assessment,
        DataQualityFindingCode::SignedIdentityWithheldByPrivacy
    ));
    assert!(!has_code(
        &assessment,
        DataQualityFindingCode::MissingSignedIdentity
    ));
    Ok(())
}

#[test]
fn durable_nonce_trace_does_not_require_last_valid_block_height()
-> Result<(), Box<dyn std::error::Error>> {
    let unsupported = fixture("unsupported-transaction-features")?.events;
    let mut events = unsupported[2..].to_vec();
    let (project_id, environment_id, trace_id) = match &events[0] {
        WireEvent::TraceCreated(event) => (
            event.project_id,
            event.environment_id,
            event
                .trace_id
                .ok_or_else(|| std::io::Error::other("durable-nonce trace is missing"))?,
        ),
        _ => return Err(std::io::Error::other("durable-nonce fixture is missing").into()),
    };
    let mut submission = fixture("identical-retry")?.events[1].clone();
    if let WireEvent::SubmissionStarted(event) = &mut submission {
        event.project_id = project_id;
        event.environment_id = environment_id;
        event.trace_id = Some(trace_id);
        event.business_action_id = None;
    } else {
        return Err(std::io::Error::other("submission fixture is missing").into());
    }
    events.push(submission);
    let ordered = order(events)?;
    let projection = project(&ordered)?;
    let grouping = group_trace(&ordered)?;
    let assessment = evaluate_data_quality(&ordered, &projection, &grouping)?;

    assert!(!has_code(
        &assessment,
        DataQualityFindingCode::MissingLastValidBlockHeight
    ));
    Ok(())
}

#[test]
fn event_permutations_produce_identical_assessments() -> Result<(), Box<dyn std::error::Error>> {
    let events = fixture("identical-retry")?.events;
    let forward = order(events.clone())?;
    let forward_assessment =
        evaluate_data_quality(&forward, &project(&forward)?, &group_trace(&forward)?)?;
    let mut reversed_events = events;
    reversed_events.reverse();
    let reversed = order(reversed_events)?;
    let reversed_assessment =
        evaluate_data_quality(&reversed, &project(&reversed)?, &group_trace(&reversed)?)?;

    assert_eq!(forward_assessment, reversed_assessment);
    Ok(())
}

#[test]
fn products_for_different_traces_cannot_be_mixed() -> Result<(), Box<dyn std::error::Error>> {
    let success = order(fixture("success")?.events)?;
    let retry = order(fixture("identical-retry")?.events)?;

    assert_eq!(
        evaluate_data_quality(&success, &project(&success)?, &group_trace(&retry)?),
        Err(DataQualityError::TraceMismatch)
    );
    Ok(())
}
