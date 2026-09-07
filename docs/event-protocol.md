# Landfall Event Protocol Catalog

- **Status:** Phase 2 Task 1 implementation baseline
- **Wire version:** `1.0`
- **Date:** 2026-09-07
- **Related decisions:** [ADR-002](adr/002-immutable-event-inputs-and-relational-projections.md), [ADR-003](adr/003-business-action-trace-attempt-event-and-alias-identifiers.md), [ADR-004](adr/004-privacy-modes-and-signed-byte-fingerprints.md), [ADR-005](adr/005-json-schema-event-contract-and-code-generation.md)

## 1. Purpose and authority

This document fixes the P0 event vocabulary and common envelope before the
individual JSON Schema resources are implemented. It defines which immutable
facts may enter the Landfall event stream, their producer and scope, and which
fields every event shares.

The versioned JSON Schema resources created in the next tasks become the
canonical machine-readable wire contract. They must implement this catalog
without silently adding, removing, or reinterpreting an event. A semantic
change to this catalog requires an explicit compatibility decision.

## 2. Protocol boundary

The raw event stream contains **evidence**, not conclusions. SDK, application,
collector, and observer components append facts that were observed at a point
in time. Events are never updated in place.

The projector consumes those facts and creates versioned relational
projections. Diagnoses and recommendations are derived records with source
event references; they are not accepted as raw events. In particular,
`landfall.diagnosis.generated` and `landfall.recommendation.generated` are not
members of the v1.0 event union.

This separation guarantees that replaying the same retained evidence with a
named reducer and rule-set version produces a deterministic result without
feeding an earlier projection back into its own input.

## 3. Common event envelope

Every v1.0 event has this conceptual shape:

```json
{
  "schema_version": "1.0",
  "event_id": "0198f0c1-1234-7abc-8def-1234567890ab",
  "event_type": "solana.submission.completed",
  "occurred_at": "2026-08-29T12:00:00.123Z",
  "monotonic_ns": "28400123",
  "project_id": "0198ef00-0000-7000-8000-000000000001",
  "environment_id": "0198ef01-0000-7000-8000-000000000001",
  "trace_id": "0198f0a0-0000-7000-8000-000000000001",
  "business_action_id": "0198f090-0000-7000-8000-000000000001",
  "source": {
    "kind": "sdk",
    "name": "landfall-js",
    "version": "0.1.0",
    "instance_id": "0198ef10-0000-7000-8000-000000000001",
    "service": "swap-worker",
    "app_version": "git:abc123"
  },
  "privacy_mode": "standard",
  "privacy_policy_version": "1.0",
  "redaction_version": "1.0",
  "attributes": {
    "attempt_id": "0198f0b0-0000-7000-8000-000000000001",
    "route_id": "0198ef20-0000-7000-8000-000000000001",
    "duration_ns": "18000000",
    "transport_result": "response_received",
    "rpc_result": "accepted"
  }
}
```

The example illustrates the envelope; the event-specific schemas own the exact
contents of `attributes`.

### 3.1 Envelope fields

| Field | Presence | Meaning and invariant |
|---|---|---|
| `schema_version` | Required | Exact registered `MAJOR.MINOR` wire version. P0 emits `1.0`; it is not inferred from an HTTP route or SDK version. |
| `event_id` | Required | Canonical lowercase UUIDv7. It is the authoritative ingestion idempotency key and remains unchanged when the same fact is retried in another batch. |
| `event_type` | Required | Closed discriminator from the v1.0 catalog. Each event schema constrains it with JSON Schema `const`. |
| `occurred_at` | Required | Producer-observed UTC RFC 3339 wall-clock time. It is evidence, not guaranteed global ordering truth. |
| `monotonic_ns` | Optional | Producer monotonic-clock reading encoded as a canonical unsigned decimal string. It is comparable only when `source.instance_id` is identical. |
| `project_id` | Required | Canonical UUIDv7 for the owning project. Authentication must independently authorize it. |
| `environment_id` | Required | Canonical UUIDv7 for the owning environment and cluster boundary. Correlation never crosses this boundary. |
| `trace_id` | Scope-dependent | Canonical UUIDv7 for one transaction trace. Required by transaction-scoped events and absent only where the scope rules permit it. |
| `business_action_id` | Optional | Canonical UUIDv7 grouping explicit replacement traces for one customer intention. It is never inferred from timing, accounts, amounts, or instruction similarity. |
| `source` | Required | Closed producer identity object described below. It identifies where the fact was observed, not a submission or RPC route. |
| `privacy_mode` | Required | Privacy mode applied before serialization. The closed values are defined in Phase 2 Task 4. |
| `privacy_policy_version` | Required | Version of the environment privacy policy applied to the event. The collector verifies it against server-side configuration. |
| `redaction_version` | Required | Version of the deterministic field-selection/redaction pipeline that produced the event. |
| `attributes` | Required | Closed, event-specific object. P0 has no arbitrary metadata or unregistered extension bag. |

`received_at`, HTTP `request_id`, and transport `batch_id` are deliberately not
event-envelope fields. The collector assigns `received_at`; the HTTP handler
assigns `request_id`; and the batch wrapper owns `batch_id`. None of them
changes the identity of an event.

### 3.2 Source object

| Field | Presence | Meaning |
|---|---|---|
| `kind` | Required | Closed producer class: `sdk`, `application`, `observer`, `collector`, or `cli`. |
| `name` | Required | Bounded component name such as `landfall-js` or `landfall-observer`. It cannot contain a URL or credential. |
| `version` | Required | Bounded producer/component version. It is separate from the event schema version. |
| `instance_id` | Conditional | UUIDv7 for one process/runtime instance. Required when `monotonic_ns` is present so values from restarts are never compared. |
| `service` | Optional | Bounded customer service label, for example `swap-worker`; cardinality policy applies. |
| `app_version` | Optional | Bounded deployment/build label used for comparisons, not an unbounded metadata field. |

Submission routes and observer RPC sources are event-specific attributes with
configured IDs. They are not represented by `source.name`, and credential-
bearing endpoint URLs are prohibited everywhere in the protocol.

### 3.3 Scope rules

| Event group | Required identity |
|---|---|
| Transaction lifecycle and observer events | `project_id`, `environment_id`, and `trace_id` |
| `solana.business_outcome.observed` | At least one of `trace_id` or `business_action_id`; both should be present when the application knows both |
| `landfall.data_quality.detected` | Always project/environment scoped; may additionally reference a business action, trace, attempt, batch, or source event according to its attributes |

`attempt_id` remains inside the attributes of submission-related events, as
established by ADR-003 and the ingestion example. Scheduling a retry does not
create a new attempt: a new `attempt_id` is created only when the next actual
submission invocation starts.

## 4. P0 event catalog

### 4.1 Application-side transaction lifecycle

| Event type | Normal producer | Immutable fact represented |
|---|---|---|
| `solana.trace.created` | SDK or application | A trace context for one prospective transaction lifecycle was created before submission. It may already reference a business action, flow, cluster, and supported transaction format. |
| `solana.blockhash.acquired` | SDK or application | A recent blockhash result and its validity evidence were acquired for this trace. It does not claim the blockhash was later signed or submitted. |
| `solana.simulation.started` | SDK or application | One simulation invocation began with its known configuration. |
| `solana.simulation.completed` | SDK or application | That simulation invocation completed with a normalized success, rejection, or transport/RPC result and bounded evidence. Simulation success does not prove execution success. |
| `solana.signing.started` | SDK or application | A signing operation began. It contains no key material, signer credentials, or transaction bytes. |
| `solana.signing.completed` | SDK or application | Signing completed or failed. When allowed and available, successful signing may provide the Solana signature and versioned HMAC signed-byte fingerprint; raw signed bytes are prohibited. |
| `solana.submission.started` | SDK or application | One application-visible submission attempt began through a configured route. This event introduces the attempt's `attempt_id`. |
| `solana.submission.completed` | SDK or application | That attempt produced a transport/RPC result, duration, and any permitted returned signature or receipt. RPC acceptance does not prove landing. |
| `solana.submission.retry_scheduled` | SDK or application | The application decided or was configured to retry later, including bounded reason and delay evidence. It does not claim the retry happened and does not create the future attempt. |
| `solana.confirmation_wait.started` | SDK or application | The application began waiting for a commitment or terminal condition. |
| `solana.confirmation_wait.completed` | SDK or application | The wait returned, timed out, was cancelled, or failed with its observed result. A timeout is an application observation, not proof of non-inclusion. |

### 4.2 Network observation and enrichment

| Event type | Normal producer | Immutable fact represented |
|---|---|---|
| `solana.status.observed` | Observer | One configured observer source returned a signature-status/block-height observation, including an explicit not-found/null observation when applicable. Every later poll is a new event. |
| `solana.execution.enriched` | Observer | A sufficiently strong observation was enriched with bounded transaction execution metadata, error, fee, compute, slot, and source evidence. It never carries raw transaction bytes or unbounded logs. |

### 4.3 Business reconciliation and telemetry quality

| Event type | Normal producer | Immutable fact represented |
|---|---|---|
| `solana.business_outcome.observed` | SDK or application | The customer application observed success, failure, timeout, cancellation, or another normalized business reconciliation result. It remains distinct from landing and execution outcome. |
| `landfall.data_quality.detected` | SDK, application, collector, observer, or CLI | A concrete capture limitation or evidence conflict was detected, such as dropped telemetry, unsupported transaction version, unavailable signed-byte fingerprint, clock issue, observer gap, or missing block height. It records the limitation; the projector determines its effect on data-quality grade and diagnosis certainty. |

There are 15 accepted v1.0 event types: 11 application-side lifecycle events,
2 observer events, 1 business-outcome event, and 1 data-quality event.

## 5. Cross-event invariants

- Every newly observed fact receives a new `event_id`; only delivery retries of
  that exact fact reuse an ID.
- Corrections and superseding observations are new events and reference earlier
  evidence where their event-specific schema permits it.
- `.started` does not guarantee `.completed`. Missing completion is meaningful
  evidence and must remain representable.
- Batch order is not authoritative event order. Reduction uses semantic links,
  comparable monotonic time, wall time, collector receive time, and `event_id`
  as the deterministic final tie-breaker.
- Identical signed bytes submitted repeatedly remain one trace with multiple
  attempt IDs. Different signed bytes remain different traces even under one
  business action.
- Signature, fingerprint, blockhash, route, and observer-source correlation is
  environment-scoped and subject to the active privacy policy.
- Unknown event types and unknown object properties are rejected. P0 does not
  preserve arbitrary fields “for later.”
- No event can contain private keys, seed phrases, signing credentials,
  credential-bearing URLs, arbitrary request/environment objects, or raw signed
  transaction bytes.

## 6. Batch transport relationship

`POST /api/v1/events:batch` wraps up to 100 events in a transport document with
`batch_id`, `sent_at`, and `events`. The batch may be retried with the same
`batch_id`, while each contained `event_id` remains the authoritative global
deduplication key.

The collector authenticates and checks compressed/decompressed limits before
schema, typed semantic, ownership, and privacy validation. One invalid event
rejects the entire batch. A successful response is returned only after new
events and their projection jobs are durably committed together.

## 7. Golden-scenario coverage

| Required scenario | Minimum event vocabulary that expresses it |
|---|---|
| Successful transaction | trace, blockhash, optional simulation, signing, submission, status observations, execution enrichment, optional business outcome |
| Simulation error | simulation started/completed with normalized failure; later signing/submission may be absent |
| Submission rejection | submission started/completed with response received and rejected RPC result |
| Transport timeout, later success | submission completion with timeout followed by observer status/execution evidence |
| Expiry without observed inclusion | blockhash validity evidence, submission attempt, repeated status/block-height observations, optional confirmation timeout |
| Compute-budget execution error | status observation plus execution enrichment with normalized compute failure |
| Repeated identical submission | one trace/fingerprint with multiple submission attempt pairs |
| Replacement transaction | multiple trace IDs and signed-byte fingerprints under one business action ID |
| Two successful replacements | two replacement traces with independent successful execution evidence under one business action |
| Observer disagreement | separate `status.observed` events from different configured observer-source IDs |
| Missing block height | status evidence plus `data_quality.detected`; no fabricated expiration conclusion |
| Unsupported nonce/version or missing fingerprint | lifecycle evidence plus `data_quality.detected`, with prohibited fields still absent |

The catalog can therefore express every Phase 2 golden incident without storing
raw signed transactions or treating a derived diagnosis as source evidence.

## 8. Deferred to the next protocol tasks

This task deliberately does not freeze every event-specific attribute. The next
tasks define:

1. closed Draft 2020-12 schemas and bounds for the batch, envelope, common
   resources, and all 15 events;
2. canonical decimal-string lexical rules and numeric domains;
3. closed enums for privacy, commitment, transport/RPC results, business
   outcomes, source results, and normalized errors;
4. Rust wire types, generated/verified TypeScript types, compatibility rules,
   privacy classifications, and shared valid/invalid fixtures.

Those tasks may refine attribute names and composition, but they may not change
the evidence meaning, identity boundaries, or raw-versus-derived distinction
without updating this catalog and its governing ADRs.
