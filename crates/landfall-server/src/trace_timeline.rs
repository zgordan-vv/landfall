//! Privacy-safe, canonically ordered event timeline for one trace.
//!
//! Raw events stay the source of truth, but they carry signatures, signed-byte
//! fingerprints, blockhashes, and free-form text that must not reach the read
//! API. This module copies only an explicit allowlist of bounded, typed
//! attributes into the response; anything not named here is dropped.

use std::collections::HashMap;

use landfall_storage::RawEventRow;
use serde::Serialize;
use serde_json::{Map, Value};
use time::format_description::well_known::Rfc3339;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::projection::order_loaded_events;

/// Ordered lifecycle evidence for one trace.
#[derive(Debug, Serialize, ToSchema)]
pub struct TraceTimeline {
    /// Trace identity.
    pub trace_id: String,
    /// `canonical` when the reducer ordering was applied; `occurred_at` when the
    /// events could not be canonically ordered and producer time was used.
    pub ordering: String,
    /// Events in the order the reducer evaluated them.
    pub events: Vec<TraceTimelineEvent>,
}

/// One allowlisted event.
#[derive(Debug, Serialize, ToSchema)]
pub struct TraceTimelineEvent {
    /// Immutable event identity, referenced by diagnostic evidence.
    pub event_id: String,
    /// Protocol event type, for example `solana.submission.completed`.
    pub event_type: String,
    /// Producer wall-clock time (RFC 3339).
    pub occurred_at: String,
    /// Source kind (`sdk`, `application`, `observer`, `collector`, `cli`).
    pub source_kind: Option<String>,
    /// Source name, for example `landfall-observer`.
    pub source_name: Option<String>,
    /// Allowlisted attributes; absent fields were not reported or not allowed.
    pub attributes: TimelineAttributes,
}

/// Bounded attributes that are safe to show to any reader of the trace.
#[derive(Debug, Default, Serialize, ToSchema, PartialEq, Eq)]
pub struct TimelineAttributes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commitment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_commitment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preflight_commitment: Option<String>,
    /// Decimal nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ns: Option<String>,
    /// Decimal nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_ns: Option<String>,
    /// Decimal nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ns: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_slot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_valid_block_height: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirmations: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units_consumed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_units_consumed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_lamports: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    /// Registered display name of `route_id`; never the endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub route_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_route_id: Option<String>,
    /// Registered display name of `next_route_id`; never the endpoint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_route_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_attempt_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempt_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_sequence: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_signatures: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_preflight: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs_present: Option<bool>,
    /// Data-quality category (`landfall.data_quality.detected` only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_impact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TimelineError>,
}

/// Normalized error without its free-form message.
#[derive(Debug, Serialize, ToSchema, PartialEq, Eq)]
pub struct TimelineError {
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_index: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_code: Option<u64>,
}

/// Builds the timeline from durable rows, using the reducer's canonical order
/// when it can be derived and producer time otherwise.
#[must_use]
pub fn build_timeline<S: std::hash::BuildHasher>(
    trace_id: Uuid,
    rows: Vec<RawEventRow>,
    route_names: &HashMap<Uuid, String, S>,
) -> TraceTimeline {
    let (ordering, ordered_rows) = if let Ok(order) = order_loaded_events(rows.clone()) {
        let mut by_id = rows
            .into_iter()
            .map(|row| (row.event_id, row))
            .collect::<HashMap<_, _>>();
        let ordered = order
            .events()
            .iter()
            .filter_map(|event| by_id.remove(&event.event_id().into_uuid()))
            .collect::<Vec<_>>();
        ("canonical", ordered)
    } else {
        let mut fallback = rows;
        fallback.sort_by_key(|row| (row.occurred_at, row.received_at, row.event_id));
        ("occurred_at", fallback)
    };
    TraceTimeline {
        trace_id: trace_id.to_string(),
        ordering: ordering.to_owned(),
        events: ordered_rows
            .into_iter()
            .map(|row| timeline_event(&row, route_names))
            .collect(),
    }
}

fn timeline_event<S: std::hash::BuildHasher>(
    row: &RawEventRow,
    route_names: &HashMap<Uuid, String, S>,
) -> TraceTimelineEvent {
    let source = row.payload.get("source").and_then(Value::as_object);
    let empty = Map::new();
    let attributes = row
        .payload
        .get("attributes")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    TraceTimelineEvent {
        event_id: row.event_id.to_string(),
        event_type: row.event_type.clone(),
        occurred_at: row.occurred_at.format(&Rfc3339).unwrap_or_default(),
        source_kind: source.and_then(|value| token(value, "kind")),
        source_name: source.and_then(|value| token(value, "name")),
        attributes: allowlisted_attributes(attributes, route_names),
    }
}

/// Copies only the allowlisted keys, each validated against its expected shape.
#[must_use]
pub fn allowlisted_attributes<S: std::hash::BuildHasher>(
    attributes: &Map<String, Value>,
    route_names: &HashMap<Uuid, String, S>,
) -> TimelineAttributes {
    let route_id = identifier(attributes, "route_id");
    let next_route_id = identifier(attributes, "next_route_id");
    let data_quality = attributes.contains_key("severity");
    TimelineAttributes {
        flow: token(attributes, "flow"),
        transaction_version: token(attributes, "transaction_version"),
        result: token(attributes, "result"),
        transport_result: token(attributes, "transport_result"),
        rpc_result: token(attributes, "rpc_result"),
        source_result: token(attributes, "source_result"),
        execution_result: token(attributes, "execution_result"),
        outcome: token(attributes, "outcome"),
        reason: token(attributes, "reason"),
        commitment: token(attributes, "commitment"),
        observed_commitment: token(attributes, "observed_commitment"),
        preflight_commitment: token(attributes, "preflight_commitment"),
        duration_ns: decimal(attributes, "duration_ns"),
        delay_ns: decimal(attributes, "delay_ns"),
        timeout_ns: decimal(attributes, "timeout_ns"),
        slot: decimal(attributes, "slot"),
        context_slot: decimal(attributes, "context_slot"),
        block_height: decimal(attributes, "block_height"),
        last_valid_block_height: decimal(attributes, "last_valid_block_height"),
        confirmations: decimal(attributes, "confirmations"),
        block_time: timestamp(attributes, "block_time"),
        units_consumed: decimal(attributes, "units_consumed"),
        compute_units_consumed: decimal(attributes, "compute_units_consumed"),
        fee_lamports: decimal(attributes, "fee_lamports"),
        route_name: route_id.and_then(|id| route_names.get(&id).cloned()),
        route_id: route_id.map(|id| id.to_string()),
        next_route_name: next_route_id.and_then(|id| route_names.get(&id).cloned()),
        next_route_id: next_route_id.map(|id| id.to_string()),
        attempt_id: identifier(attributes, "attempt_id").map(|id| id.to_string()),
        previous_attempt_id: identifier(attributes, "previous_attempt_id").map(|id| id.to_string()),
        attempt_sequence: attributes.get("attempt_sequence").and_then(Value::as_u64),
        retry_sequence: attributes.get("retry_sequence").and_then(Value::as_u64),
        max_retries: attributes.get("max_retries").and_then(Value::as_u64),
        required_signatures: attributes
            .get("required_signatures")
            .and_then(Value::as_u64),
        skip_preflight: attributes.get("skip_preflight").and_then(Value::as_bool),
        logs_present: attributes.get("logs_present").and_then(Value::as_bool),
        quality_category: data_quality
            .then(|| token(attributes, "category"))
            .flatten(),
        quality_severity: token(attributes, "severity"),
        quality_impact: token(attributes, "impact"),
        error: attributes
            .get("error")
            .and_then(Value::as_object)
            .and_then(normalized_error),
    }
}

fn normalized_error(error: &Map<String, Value>) -> Option<TimelineError> {
    Some(TimelineError {
        category: token(error, "category")?,
        code: token(error, "code"),
        instruction_index: error.get("instruction_index").and_then(Value::as_u64),
        custom_code: error.get("custom_code").and_then(Value::as_u64),
    })
}

/// Whether a producer-supplied value is a short machine token (an enum value
/// or a flow name) rather than free text.
#[must_use]
pub fn is_token(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
}

fn token(object: &Map<String, Value>, key: &str) -> Option<String> {
    let value = object.get(key)?.as_str()?;
    is_token(value).then(|| value.to_owned())
}

fn decimal(object: &Map<String, Value>, key: &str) -> Option<String> {
    let value = object.get(key)?.as_str()?;
    let valid = (1..=20).contains(&value.len()) && value.bytes().all(|byte| byte.is_ascii_digit());
    valid.then(|| value.to_owned())
}

fn timestamp(object: &Map<String, Value>, key: &str) -> Option<String> {
    let value = object.get(key)?.as_str()?;
    time::OffsetDateTime::parse(value, &Rfc3339)
        .ok()
        .and_then(|parsed| parsed.format(&Rfc3339).ok())
}

fn identifier(object: &Map<String, Value>, key: &str) -> Option<Uuid> {
    Uuid::parse_str(object.get(key)?.as_str()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn row(event_id: Uuid, event_type: &str, occurred_at: &str, attributes: &Value) -> RawEventRow {
        let occurred = time::OffsetDateTime::parse(occurred_at, &Rfc3339)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        RawEventRow {
            event_id,
            project_id: Uuid::nil(),
            environment_id: Uuid::nil(),
            trace_id: Some(Uuid::nil()),
            event_type: event_type.to_owned(),
            occurred_at: occurred,
            received_at: occurred,
            payload: json!({
                "event_type": event_type,
                "source": {"kind": "application", "name": "demo-app", "version": "1.0.0", "instance_id": "01a15a95-8b6a-7f68-a170-657dbd15d610"},
                "attributes": attributes,
            }),
        }
    }

    #[test]
    fn drops_signatures_fingerprints_blockhashes_and_free_text() {
        let attributes = json!({
            "result": "completed",
            "duration_ns": "1137750",
            "signature": "2PJL7n7AgZJefw6Nn35xfzmXtXRDTaD66r8W8b5ddbJCe6vEYDHJFZhqUmkrq2jLihFxvEyMh1EkmS9GHj33UzNp",
            "signed_bytes_fingerprint": {"key_id": "01a15a82-e18b-78a1-b68c-4c27823d332c", "algorithm": "lf-hmac-sha256-v1", "value_hex": "22a7"},
            "recent_blockhash": "32P7WLojq7YKCbiTZyFFPPRQu3Q1NwrGQm7BWwyQ9pXw",
            "route_receipt": "opaque",
            "endpoint_url": "https://rpc.invalid/?api-key=secret",
            "rpc_url": "https://rpc.invalid/",
            "headers": {"authorization": "Bearer x"},
            "metadata": {"private_key": "x"},
            "detail": "free text that may contain anything",
            "error": {"category": "transport_timeout", "code": "ETIMEDOUT", "message": "connect to https://rpc.invalid failed"}
        });
        let object = attributes.as_object().cloned().unwrap_or_default();
        let safe = allowlisted_attributes(&object, &HashMap::new());
        let serialized = serde_json::to_value(&safe).unwrap_or(Value::Null);
        let text = serialized.to_string();
        for forbidden in [
            "signature",
            "fingerprint",
            "value_hex",
            "blockhash",
            "receipt",
            "rpc.invalid",
            "Bearer",
            "free text",
            "message",
        ] {
            assert!(!text.contains(forbidden), "{forbidden} leaked: {text}");
        }
        assert!(!crate::contains_prohibited_key(&serialized));
        assert_eq!(safe.result.as_deref(), Some("completed"));
        assert_eq!(safe.duration_ns.as_deref(), Some("1137750"));
        assert_eq!(
            safe.error,
            Some(TimelineError {
                category: "transport_timeout".into(),
                code: Some("ETIMEDOUT".into()),
                instruction_index: None,
                custom_code: None,
            })
        );
    }

    #[test]
    fn rejects_values_that_do_not_match_their_shape() {
        let attributes = json!({
            "flow": "checkout transfer with spaces",
            "duration_ns": "12abc",
            "route_id": "not-a-uuid",
            "block_time": "yesterday",
            "error": {"code": "missing-category"}
        });
        let object = attributes.as_object().cloned().unwrap_or_default();
        assert_eq!(
            allowlisted_attributes(&object, &HashMap::new()),
            TimelineAttributes::default()
        );
    }

    #[test]
    fn resolves_route_names_without_endpoints() {
        let route = Uuid::from_u128(0x0198_ef00_0000_7000_8000_0000_0000_0400);
        let flaky = Uuid::from_u128(0x0198_ef00_0000_7000_8000_0000_0000_0401);
        let names = HashMap::from([(route, "devnet-public".to_owned())]);
        let attributes = json!({"route_id": flaky.to_string(), "next_route_id": route.to_string()});
        let object = attributes.as_object().cloned().unwrap_or_default();
        let safe = allowlisted_attributes(&object, &names);
        assert_eq!(safe.route_name, None);
        assert_eq!(safe.next_route_name.as_deref(), Some("devnet-public"));
    }

    #[test]
    fn falls_back_to_producer_time_for_unparseable_events() {
        let first = Uuid::from_u128(1);
        let second = Uuid::from_u128(2);
        let rows = vec![
            row(
                second,
                "solana.signing.completed",
                "2026-10-20T18:31:58Z",
                &json!({"result": "completed"}),
            ),
            row(
                first,
                "solana.trace.created",
                "2026-10-20T18:31:56Z",
                &json!({"flow": "checkout-transfer"}),
            ),
        ];
        let timeline = build_timeline(Uuid::nil(), rows, &HashMap::new());
        assert_eq!(timeline.ordering, "occurred_at");
        let types = timeline
            .events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>();
        assert_eq!(types, ["solana.trace.created", "solana.signing.completed"]);
        assert_eq!(
            timeline.events[0].source_kind.as_deref(),
            Some("application")
        );
        assert_eq!(
            timeline.events[0].attributes.flow.as_deref(),
            Some("checkout-transfer")
        );
    }

    #[test]
    fn canonical_order_matches_reducer_and_never_serializes_secrets() {
        let batch: Value = serde_json::from_str(include_str!(
            "../../../fixtures/protocol/v1/valid/all-event-types.batch.json"
        ))
        .unwrap_or(Value::Null);
        let events = batch["events"].as_array().cloned().unwrap_or_default();
        let trace_id = events[0]["trace_id"].clone();
        let mut rows = events
            .into_iter()
            .filter(|event| event["trace_id"] == trace_id)
            .map(|mut event| {
                if event["event_type"] == "solana.signing.completed" {
                    event["attributes"]["signature"] = Value::String("5".repeat(88));
                }
                let occurred = event["occurred_at"]
                    .as_str()
                    .and_then(|value| time::OffsetDateTime::parse(value, &Rfc3339).ok())
                    .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
                RawEventRow {
                    event_id: event["event_id"]
                        .as_str()
                        .and_then(|value| Uuid::parse_str(value).ok())
                        .unwrap_or_default(),
                    project_id: Uuid::nil(),
                    environment_id: Uuid::nil(),
                    trace_id: None,
                    event_type: event["event_type"].as_str().unwrap_or_default().to_owned(),
                    occurred_at: occurred,
                    received_at: occurred,
                    payload: event,
                }
            })
            .collect::<Vec<_>>();
        let expected = rows
            .iter()
            .map(|row| row.event_type.clone())
            .collect::<Vec<_>>();
        let raw = serde_json::to_string(&rows.iter().map(|row| &row.payload).collect::<Vec<_>>())
            .unwrap_or_default();
        assert!(raw.contains("signature"));
        rows.reverse();
        let timeline = build_timeline(Uuid::nil(), rows, &HashMap::new());
        assert_eq!(timeline.ordering, "canonical");
        assert_eq!(
            timeline
                .events
                .iter()
                .map(|event| event.event_type.clone())
                .collect::<Vec<_>>(),
            expected
        );
        let serialized = serde_json::to_value(&timeline).unwrap_or(Value::Null);
        let text = serialized.to_string();
        assert!(!text.contains("signature"), "{text}");
        assert!(!text.contains("fingerprint"), "{text}");
        assert!(!text.contains("recent_blockhash"), "{text}");
        assert!(!crate::contains_prohibited_key(&serialized));
    }
}
