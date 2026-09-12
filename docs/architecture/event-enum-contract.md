# Landfall Event Enum Contract

- **Status:** Phase 2 Task 4 implementation baseline
- **Wire version:** `1.0`
- **Date:** 2026-09-08
- **Canonical schema:** [`enums.schema.json`](../schemas/events/v1/1.0/common/enums.schema.json)

## 1. Purpose

This document gives the human meaning of every protocol-controlled string
enumeration in the Landfall v1.0 event contract. The JSON Schema resource is
the machine-readable authority; this document explains how producers,
collectors, projectors, and reports must interpret its values.

Closed enums prevent spelling variants such as `time_out`, `timed_out`, and
`timeout` from fragmenting metrics or changing a diagnosis. Every value is a
lowercase `snake_case` string. An unregistered value is structurally invalid;
parsers must not silently map it to `unknown`.

The explicit value `unknown` is valid only where the schema lists it. It means
the producer observed that the outcome is unknown, not that a parser received
an unfamiliar future value.

## 2. Structural recognition is not runtime support

An enum can identify a protocol concept that the current product does not yet
support operationally. For example, `privacy_mode` includes `strict` so that
configuration, capability negotiation, and retained evidence can name it
unambiguously. P0 still rejects strict-mode startup/configuration with
`LF_UNSUPPORTED_PRIVACY_MODE`, as defined by the
[P0 Support Matrix](support-matrix.md). Schema validity never bypasses that
capability check.

## 3. Identity and configuration enums

| Enum | Values | Meaning |
|---|---|---|
| `source_kind` | `sdk`, `application`, `observer`, `collector`, `cli` | Class of component that directly observed and emitted the fact. |
| `privacy_mode` | `standard`, `full`, `strict` | Privacy policy selected for the environment and applied before serialization. |
| `commitment` | `processed`, `confirmed`, `finalized` | Solana commitment level requested or observed; the field name supplies that context. |
| `transaction_version` | `legacy`, `v0`, `unsupported` | Parsed Solana transaction format, or explicit evidence that the producer could not support it. |
| `submission_encoding` | `base64`, `base58` | Encoding used for the RPC submission payload. It does not change the fingerprint input. |

## 4. Do not collapse independent result dimensions

`transport_result` and an operation-specific result answer different questions.
They remain separate so that a received RPC rejection is distinguishable from
a request whose response was never observed.

| `transport_result` value | Meaning |
|---|---|
| `response_received` | A response reached the producer; inspect the operation-specific result. |
| `timeout` | The configured transport deadline elapsed without a usable response. |
| `connection_failed` | DNS, TLS, socket, or equivalent connection setup/use failed. |
| `cancelled` | The caller or local control flow cancelled the transport operation. |

Operation-specific enums are:

| Enum | Values |
|---|---|
| `blockhash_result` | `acquired`, `not_found`, `rpc_error`, `transport_error`, `malformed_response` |
| `simulation_rpc_result` | `succeeded`, `execution_error`, `blockhash_not_found`, `node_error`, `malformed_response`, `not_observed` |
| `signing_result` | `completed`, `rejected`, `timeout`, `failed`, `cancelled` |
| `submission_rpc_result` | `accepted`, `rejected`, `blockhash_rejected`, `duplicate_signature`, `already_processed`, `rate_limited`, `unauthorized`, `malformed_response`, `not_observed`, `outcome_ambiguous` |
| `confirmation_wait_result` | `commitment_reached`, `timeout`, `cancelled`, `failed` |
| `status_source_result` | `found`, `not_found`, `unavailable`, `malformed_response` |

`not_observed` means that no RPC result evidence is available, commonly because
transport did not return a usable response. `outcome_ambiguous` is stronger: the
submission may have reached the node, but the producer cannot safely classify
whether it was accepted. Neither value means transaction failure.

## 5. Chain and business outcomes

| Enum | Values | Meaning |
|---|---|---|
| `execution_result` | `success`, `failure` | On-chain execution result from enriched transaction evidence. |
| `business_outcome` | `success`, `failure`, `timeout`, `cancelled`, `unknown` | Application-observed outcome for a trace or business action. |

These are intentionally separate. A transaction can execute successfully while
the surrounding business action fails, or multiple replacement traces can
contribute evidence to one business outcome.

## 6. Data-quality enums

`data_quality_category` identifies the problem:

- `telemetry_dropped`, `clock_issue`, `source_incomplete`;
- `observer_gap`, `observer_disagreement`, `block_height_missing`;
- `unsupported_transaction_version`, `unsupported_durable_nonce`;
- `fingerprint_unavailable`, `business_action_conflict`,
  `privacy_field_omitted`.

`data_quality_severity` is `info`, `warning`, or `error`.
`data_quality_impact` records the analytical consequence independently:
`none`, `reduced_completeness`, `reduced_certainty`,
`correlation_unavailable`, `expiration_analysis_unavailable`, or
`observation_incomplete`.

Keeping category, severity, and impact separate lets policy change how a known
condition is prioritized without rewriting the original condition.

## 7. Normalized errors

`normalized_error_category` provides a stable analytical category while the
bounded redacted source code/message remains optional evidence. Its values are
grouped here by responsibility:

- transaction shape/support: `invalid_transaction`, `missing_signature`,
  `transaction_too_large`, `unsupported_transaction_version`,
  `unsupported_durable_nonce`;
- blockhash/execution: `blockhash_not_found`, `blockhash_expired`,
  `instruction_error`, `compute_budget_exceeded`, `insufficient_funds`,
  `account_in_use`, `slippage_exceeded`, `custom_program_error`;
- signer: `signer_rejected`, `signer_timeout`, `signer_error`;
- RPC: `rpc_rejected`, `rate_limited`, `unauthorized`,
  `duplicate_signature`, `already_processed`;
- transport/observation: `transport_timeout`, `dns_error`, `tls_error`,
  `connection_error`, `malformed_response`, `cancelled`,
  `observer_unavailable`;
- `unknown`: the producer observed an error but could not safely normalize it
  into a more specific registered category.

## 8. Cross-field semantic rules

JSON Schema validates each closed vocabulary and the event's structural shape.
The typed semantic layer must additionally reject contradictions, for example:

- `transport_result = response_received` with an RPC result of `not_observed`;
- `transport_result != response_received` with an RPC result that claims
  `accepted` or `succeeded`;
- `confirmation_wait_result = commitment_reached` without an
  `observed_commitment`;
- a successful operation carrying an error object that claims it failed.

These rules belong to semantic validation because they relate multiple fields
and may evolve with product capability; no component may infer a different
meaning merely because each field passes schema validation independently.

## 9. Evolution and storage

Adding, removing, renaming, or reinterpreting an enum value requires an explicit
compatibility review under ADR-005. Exhaustive consumers must fail visibly when
they encounter an unsupported protocol version, not accept a future value as a
current one.

PostgreSQL stores these values as bounded text constrained by application/schema
validation rather than native PostgreSQL enum types. That keeps protocol-version
migrations and replay of older raw events explicit instead of coupling wire
evolution to irreversible database enum changes.
