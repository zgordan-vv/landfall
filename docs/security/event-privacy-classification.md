# Event examples and field-level privacy classification

- **Status:** Phase 2 Task 10 baseline
- **Event schema version:** `1.0`
- **Privacy classification version:** `1.0`
- **Date:** 2026-09-09
- **Machine-readable authority:** [`event-privacy-classification.json`](event-privacy-classification.json)
- **Governing decision:** [ADR-004](adr/004-privacy-modes-and-signed-byte-fingerprints.md)

## 1. Purpose

This document explains what v1 event payloads look like and how every field is
handled before central persistence. The JSON Schema remains authoritative for
wire shape. The privacy registry is authoritative for field classification and
is checked against every schema by CI; this document explains the policy in
human-readable form.

A field being public on Solana does not make a collection of that field harmless.
Signatures, blockhashes, routes, timestamps, failures, and deployment labels can
together reveal customer activity and strategy. Consequently, all event access,
export, retention, and deletion remains environment-scoped.

## 2. Classification and mode behavior

| Class | Sensitivity | Standard P0 | Full P0 | Strict P1 | Reason |
|---|---:|---|---|---|---|
| `container` | Structural | Validate children | Validate children | Validate children | An object/array has no independent policy; each child owns one. |
| `system_identifier` | Moderate | Store | Store | Store | Landfall UUIDs enable ownership, idempotency, causality, and replay. They remain tenant data. |
| `customer_context` | Moderate | Store bounded | Store bounded | Store bounded | Controlled service, deployment, flow, reason, and receipt labels aid diagnosis but may expose architecture. |
| `timing` | Moderate | Store | Store | Store | Ordering and latency are product evidence but can reveal traffic patterns. |
| `normalized_telemetry` | Low | Store | Store | Store | Closed enums, booleans, versions, and bounded counters describe behavior without raw payloads. |
| `public_chain_identifier` | High | Store | Store | Local only | Signatures and recent blockhashes permit public-chain lookup and correlation. |
| `public_chain_measurement` | Moderate | Store | Store | Store | Slots, heights, commitments, fees, and compute measurements are needed for diagnosis. |
| `pseudonymous_fingerprint` | High | Store scoped | Store scoped | Store scoped | HMAC equality is useful only with environment, algorithm, and key ID; never expose it as a global digest. |
| `bounded_free_text` | High | Redact, then store | Redact, then store | Redact, then store | Allowed diagnostic text can still contain credentials or URLs and must pass deterministic redaction. |

`full` does not make arbitrary data acceptable. It only permits future,
explicitly allowlisted public account/program fields; v1 currently defines none.
Raw transaction bytes and secrets remain prohibited in every mode.

## 3. Shared transport and envelope fields

| Scope | Field | Class | Notes |
|---|---|---|---|
| Batch | `batch_id` | `system_identifier` | Transport retry identity; not event identity. |
| Batch | `sent_at` | `timing` | Producer send time. |
| Batch | `events` | `container` | Each element is independently classified. |
| Envelope | `schema_version` | `normalized_telemetry` | Exact wire version, currently `1.0`. |
| Envelope | `event_id` | `system_identifier` | Global event idempotency key. |
| Envelope | `event_type` | `normalized_telemetry` | Closed discriminator. |
| Envelope | `occurred_at` | `timing` | Producer wall-clock evidence. |
| Envelope | `monotonic_ns` | `timing` | Comparable only within one source instance. |
| Envelope | `project_id` | `system_identifier` | Authorization must independently verify ownership. |
| Envelope | `environment_id` | `system_identifier` | Privacy and correlation boundary. |
| Envelope | `trace_id` | `system_identifier` | Transaction-lifecycle identity. |
| Envelope | `business_action_id` | `system_identifier` | Groups explicit replacement traces. |
| Envelope | `source` | `container` | Uses the source-field policy below. |
| Envelope | `privacy_mode` | `normalized_telemetry` | Declared mode; collector checks server policy. |
| Envelope | `privacy_policy_version` | `normalized_telemetry` | Applied environment policy version. |
| Envelope | `redaction_version` | `normalized_telemetry` | Applied redaction implementation version. |
| Envelope | `attributes` | `container` | Closed shape selected by `event_type`. |

### Source identity

| Field | Class | Notes |
|---|---|---|
| `kind` | `normalized_telemetry` | Closed producer class. |
| `name`, `version` | `customer_context` | Bounded component identity; never an endpoint URL. |
| `instance_id` | `system_identifier` | Separates monotonic clocks across processes/restarts. |
| `service`, `app_version` | `customer_context` | Optional bounded operational labels; avoid user or wallet identity. |

### Normalized error

| Field | Class | Notes |
|---|---|---|
| `category`, `custom_code` | `normalized_telemetry` | Closed category and bounded numeric program code. |
| `code` | `customer_context` | Bounded machine-oriented code; it must not contain a raw error object. |
| `instruction_index` | `public_chain_measurement` | Numeric execution location without instruction content. |
| `message` | `bounded_free_text` | Replace the complete field with `[REDACTED]` when a secret pattern is detected. |

### Signed-byte fingerprint

| Field | Class | Notes |
|---|---|---|
| `algorithm` | `normalized_telemetry` | Must be `lf-hmac-sha256-v1` in P0. |
| `key_id` | `system_identifier` | Non-secret environment key-version ID. |
| `value_hex` | `pseudonymous_fingerprint` | Store and compare only as `(environment_id, algorithm, key_id, value_hex)`. |

## 4. Event attribute fields

Every attribute defined by all 15 v1 event schemas appears below. A repeated
`error` or `signed_bytes_fingerprint` container inherits the shared child-field
classification above.

| Event type | Fields grouped by privacy class |
|---|---|
| `solana.trace.created` | Customer context: `flow`. Normalized telemetry: `transaction_version`, `uses_durable_nonce`. |
| `solana.blockhash.acquired` | System identifier: `route_id`. Timing: `duration_ns`. Normalized telemetry: `result`. Public-chain identifier: `recent_blockhash`. Public-chain measurement: `last_valid_block_height`, `context_slot`. Container: `error`. |
| `solana.simulation.started` | System identifiers: `simulation_id`, `route_id`. Public-chain measurements: `commitment`, `min_context_slot`. Normalized telemetry: `sig_verify`, `replace_recent_blockhash`. |
| `solana.simulation.completed` | System identifiers: `simulation_id`, `route_id`. Timing: `duration_ns`. Normalized telemetry: `transport_result`, `rpc_result`, `logs_present`. Public-chain measurement: `units_consumed`. Container: `error`. |
| `solana.signing.started` | System identifier: `signing_id`. Normalized telemetry: `transaction_version`, `required_signatures`. |
| `solana.signing.completed` | System identifier: `signing_id`. Timing: `duration_ns`. Normalized telemetry: `result`. Public-chain identifier: `signature`. Containers: `signed_bytes_fingerprint`, `error`. |
| `solana.submission.started` | System identifiers: `attempt_id`, `route_id`. Normalized telemetry: `attempt_sequence`, `encoding`, `skip_preflight`, `max_retries`. Public-chain measurements: `preflight_commitment`, `min_context_slot`. |
| `solana.submission.completed` | System identifiers: `attempt_id`, `route_id`. Timing: `duration_ns`. Normalized telemetry: `transport_result`, `rpc_result`. Public-chain identifier: `signature`. Customer context: `route_receipt`. Container: `error`. |
| `solana.submission.retry_scheduled` | System identifiers: `previous_attempt_id`, `next_route_id`. Normalized telemetry: `retry_sequence`. Timing: `delay_ns`. Customer context: `reason`. |
| `solana.confirmation_wait.started` | System identifier: `wait_id`. Public-chain measurement: `commitment`. Timing: `timeout_ns`. Normalized telemetry: `search_transaction_history`. |
| `solana.confirmation_wait.completed` | System identifier: `wait_id`. Timing: `duration_ns`. Normalized telemetry: `result`. Public-chain measurement: `observed_commitment`. Container: `error`. |
| `solana.status.observed` | System identifier: `observer_source_id`. Timing: `duration_ns`. Normalized telemetry: `source_result`. Public-chain identifier: `signature`. Public-chain measurements: `commitment`, `slot`, `confirmations`, `block_height`. Container: `error`. |
| `solana.execution.enriched` | System identifier: `observer_source_id`. Public-chain identifier: `signature`. Public-chain measurements: `slot`, `commitment`, `fee_lamports`, `compute_units_consumed`. Timing: `block_time`. Normalized telemetry: `execution_result`, `logs_present`. Container: `error`. |
| `solana.business_outcome.observed` | Normalized telemetry: `outcome`. Customer context: `reason`. Bounded free text: `detail`. |
| `landfall.data_quality.detected` | System identifiers: `attempt_id`, `batch_id`, `source_event_id`. Normalized telemetry: `category`, `severity`, `impact`. Customer context: `field`. Bounded free text: `detail`. |

The envelope-level `trace_id` and `business_action_id` are deliberately not
repeated as attributes. Their conditional presence is enforced by each event
schema.

## 5. Worked examples

### 5.1 Successful signing without transaction custody

This is the exact `attributes` object from the canonical identical-retry
fixture; the common envelope is omitted here only to focus on privacy handling.

```json
{
  "signing_id": "0198ef10-0007-7000-8000-000000000200",
  "duration_ns": "7000000",
  "result": "completed",
  "signed_bytes_fingerprint": {
    "algorithm": "lf-hmac-sha256-v1",
    "key_id": "0198ef10-0007-7000-8000-000000000300",
    "value_hex": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  }
}
```

The signature is centrally stored in standard/full mode but rejected from a
strict-mode central event. The HMAC fingerprint is retained in every mode for
environment-scoped retry/replacement correlation. Exact signed bytes are never
part of the event. The complete canonical lifecycle is the
[`success` golden fixture](../fixtures/protocol/v1/valid/incidents/success.batch.json).

### 5.2 A timeout is evidence, not a conclusion

A submission may record `transport_result: "timeout"` and a redacted normalized
error, while a later observer event records the signature as landed. Landfall
keeps both immutable observations; it does not rewrite the timeout into success.
See the canonical
[`timeout-later-success` fixture](../fixtures/protocol/v1/valid/incidents/timeout-later-success.batch.json).

### 5.3 Secret found inside an allowed string

```json
{
  "error": {
    "category": "transport",
    "message": "[REDACTED]"
  }
}
```

The original bounded `message` can be structurally valid and still contain a
bearer token, credential-bearing RPC URL, cookie, or key marker. The privacy
pipeline replaces the entire affected text field instead of attempting partial
masking. Executable input/output examples live in the
[`privacy/redact` fixtures](../fixtures/protocol/privacy/redact/).

### 5.4 All event shapes and incident narratives

- [`all-event-types.batch.json`](../fixtures/protocol/v1/valid/all-event-types.batch.json)
  is the compact canonical example of all 15 event shapes.
- [`valid/incidents`](../fixtures/protocol/v1/valid/incidents/) contains the 12
  end-to-end evidence narratives used by later reducer tests.
- [`privacy/reject`](../fixtures/protocol/privacy/reject/) demonstrates fields
  and sizes that must be rejected before typed persistence.

## 6. Prohibited data and enforcement

The following property names are forbidden in v1: `private_key`, `seed_phrase`,
`signed_transaction_bytes`, `raw_transaction`, `authorization`, `cookie`,
`set_cookie`, `headers`, `metadata`, `endpoint_url`, and `rpc_url`.

The list is illustrative of the closed-schema boundary, not permission to add a
differently named escape hatch. Arbitrary metadata, request objects, environment
objects, full logs, account/program lists, transaction objects, and credential-
bearing URLs are absent from the schema and therefore rejected.

Enforcement order is:

1. The SDK selects typed allowlisted fields and applies the configured mode.
2. The SDK redacts bounded free text before serialization.
3. JSON Schema and typed decoding reject unknown fields and hard size violations.
4. The collector independently verifies the environment mode and policy version,
   re-redacts text, and rejects mode violations before persistence.
5. Logs record only field paths and stable rejection/redaction categories—not
   rejected values, fingerprints, signatures, tokens, URLs, or cookies.

Exports and deletion cover raw events, projections, diagnoses, search/index
copies, reports, and backups according to the retention policy. Sanitized reports
must not restore local-only fields or expose pseudonymous fingerprints.

## 7. Change rule

Adding or renaming a schema property requires the same change to
`event-privacy-classification.json`. CI fails if an envelope, shared-object, or event
attribute field is missing, stale, assigned to an unknown class, or violates a
mode invariant. Changing a class action or introducing a new privacy mode is a
policy change and requires review against ADR-004, fixtures, collector behavior,
exports, retention, and deletion.
