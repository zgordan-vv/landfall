//! Cross-language wire-contract behavior exercised through Serde JSON boundaries.

use std::{collections::BTreeSet, error::Error, fs, path::PathBuf};

use landfall_protocol::{
    AttemptSequence, BlockhashResult, BusinessOutcome, Commitment, ComponentName,
    ConfirmationWaitResult, DataQualityCategory, DataQualityImpact, DataQualitySeverity,
    DurationNs, EventBatch, ExecutionResult, FingerprintHex, Int64Decimal, JsonPointer, MaxRetries,
    NormalizedErrorCategory, PrivacyMode, ProtocolVersion, RequiredSignatures, SigningResult,
    SimulationRpcResult, SolanaBlockhash, SolanaSignature, SourceKind, StatusSourceResult,
    SubmissionEncoding, SubmissionRpcResult, Token, TraceId, TransactionVersion, TransportResult,
    UtcTimestamp, WireEvent, WireValueError, WireValueErrorKind,
};
use serde::Serialize;
use serde_json::{Value, json};

const UUID: &str = "0198ef00-0000-7000-8000-000000000001";

fn event(event_type: &str, attributes: &Value) -> Value {
    json!({
        "schema_version": "1.0",
        "event_id": UUID,
        "event_type": event_type,
        "occurred_at": "2026-08-29T12:00:00.123Z",
        "project_id": UUID,
        "environment_id": UUID,
        "trace_id": UUID,
        "source": {
            "kind": "sdk",
            "name": "landfall-js",
            "version": "0.1.0"
        },
        "privacy_mode": "standard",
        "privacy_policy_version": "1.0",
        "redaction_version": "1.0",
        "attributes": attributes
    })
}

#[allow(clippy::too_many_lines)]
fn valid_events() -> Vec<Value> {
    vec![
        event(
            "solana.trace.created",
            &json!({"flow": "swap", "transaction_version": "v0"}),
        ),
        event(
            "solana.blockhash.acquired",
            &json!({"route_id": UUID, "result": "acquired", "duration_ns": "12"}),
        ),
        event(
            "solana.simulation.started",
            &json!({
                "simulation_id": UUID,
                "route_id": UUID,
                "commitment": "processed",
                "replace_recent_blockhash": true,
                "sig_verify": false
            }),
        ),
        event(
            "solana.simulation.completed",
            &json!({
                "simulation_id": UUID,
                "route_id": UUID,
                "duration_ns": "20",
                "transport_result": "response_received",
                "rpc_result": "succeeded"
            }),
        ),
        event(
            "solana.signing.started",
            &json!({"signing_id": UUID, "transaction_version": "legacy"}),
        ),
        event(
            "solana.signing.completed",
            &json!({"signing_id": UUID, "duration_ns": "7", "result": "completed"}),
        ),
        event(
            "solana.submission.started",
            &json!({
                "attempt_id": UUID,
                "route_id": UUID,
                "attempt_sequence": 1,
                "encoding": "base64",
                "skip_preflight": false
            }),
        ),
        event(
            "solana.submission.completed",
            &json!({
                "attempt_id": UUID,
                "route_id": UUID,
                "duration_ns": "30",
                "transport_result": "response_received",
                "rpc_result": "accepted"
            }),
        ),
        event(
            "solana.submission.retry_scheduled",
            &json!({
                "previous_attempt_id": UUID,
                "retry_sequence": 1,
                "delay_ns": "1000000",
                "reason": "transport_timeout"
            }),
        ),
        event(
            "solana.confirmation_wait.started",
            &json!({
                "wait_id": UUID,
                "commitment": "confirmed",
                "timeout_ns": "30000000000",
                "search_transaction_history": false
            }),
        ),
        event(
            "solana.confirmation_wait.completed",
            &json!({
                "wait_id": UUID,
                "duration_ns": "100",
                "result": "commitment_reached",
                "observed_commitment": "confirmed"
            }),
        ),
        event(
            "solana.status.observed",
            &json!({"observer_source_id": UUID, "source_result": "not_found", "duration_ns": "9"}),
        ),
        event(
            "solana.execution.enriched",
            &json!({
                "observer_source_id": UUID,
                "slot": "18446744073709551615",
                "commitment": "finalized",
                "execution_result": "success",
                "logs_present": true
            }),
        ),
        event(
            "solana.business_outcome.observed",
            &json!({"outcome": "success", "reason": "order_filled"}),
        ),
        event(
            "landfall.data_quality.detected",
            &json!({
                "category": "observer_gap",
                "severity": "warning",
                "impact": "reduced_certainty"
            }),
        ),
    ]
}

fn schema_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/events/v1")
        .join(relative)
}

fn assert_enum<T: Serialize>(
    schema: &Value,
    name: &str,
    values: &[T],
) -> Result<(), Box<dyn Error>> {
    assert_eq!(serde_json::to_value(values)?, schema["$defs"][name]["enum"]);
    Ok(())
}

#[test]
fn rust_enums_exactly_match_the_canonical_schema() -> Result<(), Box<dyn Error>> {
    let schema: Value = serde_json::from_str(&fs::read_to_string(schema_path(
        "1.0/common/enums.schema.json",
    ))?)?;

    assert_enum(&schema, "source_kind", SourceKind::ALL)?;
    assert_enum(&schema, "privacy_mode", PrivacyMode::ALL)?;
    assert_enum(&schema, "commitment", Commitment::ALL)?;
    assert_enum(&schema, "transaction_version", TransactionVersion::ALL)?;
    assert_enum(&schema, "submission_encoding", SubmissionEncoding::ALL)?;
    assert_enum(&schema, "transport_result", TransportResult::ALL)?;
    assert_enum(&schema, "blockhash_result", BlockhashResult::ALL)?;
    assert_enum(&schema, "simulation_rpc_result", SimulationRpcResult::ALL)?;
    assert_enum(&schema, "signing_result", SigningResult::ALL)?;
    assert_enum(&schema, "submission_rpc_result", SubmissionRpcResult::ALL)?;
    assert_enum(
        &schema,
        "confirmation_wait_result",
        ConfirmationWaitResult::ALL,
    )?;
    assert_enum(&schema, "status_source_result", StatusSourceResult::ALL)?;
    assert_enum(&schema, "execution_result", ExecutionResult::ALL)?;
    assert_enum(&schema, "business_outcome", BusinessOutcome::ALL)?;
    assert_enum(&schema, "data_quality_category", DataQualityCategory::ALL)?;
    assert_enum(&schema, "data_quality_severity", DataQualitySeverity::ALL)?;
    assert_enum(&schema, "data_quality_impact", DataQualityImpact::ALL)?;
    assert_enum(
        &schema,
        "normalized_error_category",
        NormalizedErrorCategory::ALL,
    )?;
    Ok(())
}

#[test]
fn rust_event_union_exactly_matches_the_manifest() -> Result<(), Box<dyn Error>> {
    let manifest: Value = serde_json::from_str(&fs::read_to_string(schema_path("manifest.json"))?)?;
    let manifest_types = manifest["event_types"]
        .as_object()
        .ok_or("manifest event_types is not an object")?
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let events = valid_events();
    let rust_types = events
        .iter()
        .map(|event| {
            event["event_type"]
                .as_str()
                .ok_or("test event_type is not a string")
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    assert_eq!(rust_types, manifest_types);
    Ok(())
}

#[test]
fn all_registered_events_round_trip_without_null_fields() -> Result<(), Box<dyn Error>> {
    for original in valid_events() {
        let decoded: WireEvent = serde_json::from_value(original.clone())?;
        let encoded = serde_json::to_value(decoded)?;
        assert_eq!(encoded, original);
    }
    Ok(())
}

#[test]
fn optional_structured_evidence_round_trips() -> Result<(), Box<dyn Error>> {
    let mut simulation = event(
        "solana.simulation.completed",
        &json!({
            "simulation_id": UUID,
            "route_id": UUID,
            "duration_ns": "20",
            "transport_result": "response_received",
            "rpc_result": "execution_error",
            "units_consumed": "18446744073709551615",
            "logs_present": true,
            "error": {
                "category": "custom_program_error",
                "code": "ProgramError",
                "message": "redacted bounded evidence",
                "instruction_index": 255,
                "custom_code": 4_294_967_295_u32
            }
        }),
    );
    simulation["monotonic_ns"] = json!("42");
    simulation["business_action_id"] = json!(UUID);
    simulation["source"] = json!({
        "kind": "application",
        "name": "swap-worker",
        "version": "git:abc123",
        "instance_id": UUID,
        "service": "transaction-service",
        "app_version": "release-2026.09"
    });

    let signature = "1".repeat(87);
    let signing = event(
        "solana.signing.completed",
        &json!({
            "signing_id": UUID,
            "duration_ns": "7",
            "result": "completed",
            "signature": signature,
            "signed_bytes_fingerprint": {
                "algorithm": "lf-hmac-sha256-v1",
                "key_id": UUID,
                "value_hex": "a".repeat(64)
            }
        }),
    );

    for original in [simulation, signing] {
        let decoded: WireEvent = serde_json::from_value(original.clone())?;
        assert_eq!(serde_json::to_value(decoded)?, original);
    }
    Ok(())
}

#[test]
fn scalar_newtypes_enforce_wire_rules() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        "18446744073709551615".parse::<DurationNs>()?.get(),
        u64::MAX
    );
    assert_eq!(
        "-9223372036854775808".parse::<Int64Decimal>()?.get(),
        i64::MIN
    );
    assert_eq!(
        "999999999.999999999".parse::<ProtocolVersion>()?.as_str(),
        "999999999.999999999"
    );
    assert_eq!(
        "2026-08-29T12:00:00.123456789Z"
            .parse::<UtcTimestamp>()?
            .to_string(),
        "2026-08-29T12:00:00.123456789Z"
    );
    assert_eq!(UUID.parse::<TraceId>()?.to_string(), UUID);
    assert_eq!(AttemptSequence::try_from(10_000)?.get(), 10_000);
    assert_eq!(MaxRetries::try_from(1_000)?.get(), 1_000);
    assert_eq!(RequiredSignatures::try_from(64)?.get(), 64);
    assert_eq!("swap.route-1".parse::<Token>()?.as_str(), "swap.route-1");
    assert_eq!(
        "landfall-js".parse::<ComponentName>()?.as_str(),
        "landfall-js"
    );
    assert_eq!(
        "1".repeat(87).parse::<SolanaSignature>()?.as_str().len(),
        87
    );
    assert_eq!(
        "1".repeat(32).parse::<SolanaBlockhash>()?.as_str().len(),
        32
    );
    assert_eq!("a".repeat(64).parse::<FingerprintHex>()?.as_str().len(), 64);
    assert_eq!(
        "/attributes/error~1code".parse::<JsonPointer>()?.as_str(),
        "/attributes/error~1code"
    );

    let overflow = "18446744073709551616".parse::<DurationNs>();
    assert_eq!(
        overflow.as_ref().err().map(WireValueError::kind),
        Some(WireValueErrorKind::OutOfRange)
    );
    assert!("01".parse::<DurationNs>().is_err());
    assert!("-0".parse::<Int64Decimal>().is_err());
    assert!("1.01".parse::<ProtocolVersion>().is_err());
    assert!("2026-08-29T12:00:00+00:00".parse::<UtcTimestamp>().is_err());
    assert!(
        "0198ef00-0000-6000-8000-000000000001"
            .parse::<TraceId>()
            .is_err()
    );
    assert!(AttemptSequence::try_from(0).is_err());
    assert!(MaxRetries::try_from(1_001).is_err());
    assert!(RequiredSignatures::try_from(0).is_err());
    assert!("Swap".parse::<Token>().is_err());
    assert!("/bad~pointer".parse::<JsonPointer>().is_err());
    Ok(())
}

#[test]
fn closed_and_bounded_wire_shapes_reject_invalid_json() {
    let mut unknown_enum = valid_events()[0].clone();
    unknown_enum["attributes"]["transaction_version"] = json!("future_v2");
    assert!(serde_json::from_value::<WireEvent>(unknown_enum).is_err());

    let mut unknown_attribute = valid_events()[0].clone();
    unknown_attribute["attributes"]["extra"] = json!(true);
    assert!(serde_json::from_value::<WireEvent>(unknown_attribute).is_err());

    let mut unknown_envelope_field = valid_events()[0].clone();
    unknown_envelope_field["secret"] = json!("must-not-enter-storage");
    assert!(serde_json::from_value::<WireEvent>(unknown_envelope_field).is_err());

    let mut unknown_event = valid_events()[0].clone();
    unknown_event["event_type"] = json!("solana.future.event");
    assert!(serde_json::from_value::<WireEvent>(unknown_event).is_err());

    let mut unsafe_integer = valid_events()[12].clone();
    unsafe_integer["attributes"]["slot"] = json!(9_007_199_254_740_992_u64);
    assert!(serde_json::from_value::<WireEvent>(unsafe_integer).is_err());
}

#[test]
fn envelope_and_result_semantics_reject_contradictions() -> Result<(), Box<dyn Error>> {
    let mut missing_trace = valid_events()[0].clone();
    missing_trace
        .as_object_mut()
        .ok_or("test event is not an object")?
        .remove("trace_id");
    assert!(serde_json::from_value::<WireEvent>(missing_trace).is_err());

    let mut monotonic_without_instance = valid_events()[0].clone();
    monotonic_without_instance["monotonic_ns"] = json!("1");
    assert!(serde_json::from_value::<WireEvent>(monotonic_without_instance).is_err());

    let mut contradictory_simulation = valid_events()[3].clone();
    contradictory_simulation["attributes"]["transport_result"] = json!("timeout");
    assert!(serde_json::from_value::<WireEvent>(contradictory_simulation).is_err());

    let mut contradictory_submission = valid_events()[7].clone();
    contradictory_submission["attributes"]["transport_result"] = json!("connection_failed");
    assert!(serde_json::from_value::<WireEvent>(contradictory_submission).is_err());

    let mut missing_commitment = valid_events()[10].clone();
    missing_commitment["attributes"]
        .as_object_mut()
        .ok_or("test attributes are not an object")?
        .remove("observed_commitment");
    assert!(serde_json::from_value::<WireEvent>(missing_commitment).is_err());
    Ok(())
}

#[test]
fn batch_enforces_collection_bounds() -> Result<(), Box<dyn Error>> {
    let event: WireEvent = serde_json::from_value(valid_events().remove(0))?;
    let batch = EventBatch::new(UUID.parse()?, "2026-08-29T12:00:00Z".parse()?, vec![event])?;
    let encoded = serde_json::to_value(batch)?;
    assert_eq!(encoded["events"].as_array().map(Vec::len), Some(1));

    let empty = json!({"batch_id": UUID, "sent_at": "2026-08-29T12:00:00Z", "events": []});
    assert!(serde_json::from_value::<EventBatch>(empty).is_err());

    let too_many = json!({
        "batch_id": UUID,
        "sent_at": "2026-08-29T12:00:00Z",
        "events": (0..101).map(|_| valid_events().remove(0)).collect::<Vec<_>>()
    });
    assert!(serde_json::from_value::<EventBatch>(too_many).is_err());
    Ok(())
}
