# ADR-005: JSON Schema event contract and code generation

- **Status:** Accepted
- **Date:** 2026-08-30
- **Decision owners:** Landfall maintainers
- **Related requirements:** FR-SDK-003, FR-SDK-006, FR-INGEST-001 through FR-INGEST-003, event envelope/types/rules, schema compatibility, security fixtures
- **Related ADRs:** [ADR-002](002-immutable-event-inputs-and-relational-projections.md), [ADR-003](003-business-action-trace-attempt-event-and-alias-identifiers.md), [ADR-004](004-privacy-modes-and-signed-byte-fingerprints.md); ADR-008 (planned)
- **Supersedes:** None
- **Superseded by:** None

## Context

Landfall's event protocol crosses several independently built boundaries: a
TypeScript SDK creates events, Rust collector/CLI processes them, PostgreSQL
retains their original shape, fixtures and reports replay them, and future
adapters may be implemented in other languages. A field name, unit, integer
representation, enum, or privacy restriction that differs across these
boundaries can silently corrupt lifecycle state and metrics.

Rust-first types would make the main Node.js producer depend on artifacts derived
from an implementation language. TypeScript-first types would couple the server
contract to the SDK. Hand-maintaining both guarantees drift. OpenAPI covers the
HTTP resource API but is not an independent versioned source for files, manual
events, observer evidence, and future non-HTTP transports.

The protocol needs one language-neutral source of truth, discriminated event
types, strict bounds, backward-compatibility rules, deterministic generation,
runtime validation, and cross-language fixtures. It must also distinguish
structural validity from authorization, privacy, cross-field semantics, and
domain interpretation that JSON Schema alone cannot safely express.

## Decision drivers

- Rust and TypeScript must accept and reject the same wire documents.
- Every event type needs explicit required fields, enum values, units, sizes, and
  unknown-field behavior.
- JSON integers beyond JavaScript's safe range must preserve exact values.
- Generated artifacts must be deterministic, reviewable, and available without a
  network or code generator during a normal build.
- Collector validation is authoritative; TypeScript compile-time types are not a
  security boundary.
- Event contracts must remain usable by manual producers and future languages.
- Schema evolution must not silently change the meaning of retained raw events.
- Privacy allowlists and denial of arbitrary nested metadata must be expressible
  and tested.
- Validation must fit the bounded 100-event/1-MiB ingestion request and 250-ms
  p95 target.
- Public validation errors must identify a safe location/category without
  reflecting secrets or complete rejected values.
- REST/OpenAPI evolution and event-protocol evolution must remain separate but
  composable.

## Options considered

### Option A — JSON Schema 2020-12 as canonical wire contract

Hand-author versioned JSON Schema resources, generate TypeScript wire DTOs, keep
ergonomic Rust Serde wire types, and make both implementations pass the same
runtime schema validation and fixture corpus. Commit schemas, fixtures, generated
TypeScript, and generation metadata.

This is language-neutral, human-readable, appropriate for JSON/HTTP and NDJSON,
and supports strict object composition with `$defs`, `$ref`, `oneOf`, `const`,
and `unevaluatedProperties`. It requires schema discipline, a generation pipeline,
and an explicit semantic-validation layer beyond the schema.

### Option B — Rust types as source and generate schemas with Schemars

Author Serde/Schemars structs and generate JSON Schema, then TypeScript. This
makes the collector implementation convenient but lets Rust annotations define a
cross-language protocol. Generated schema diffs can be difficult to shape,
schema-only constraints may need custom annotations, and SDK contributors must
understand Rust changes to evolve their primary contract.

Rust types remain a verified implementation, not the authority.

### Option C — TypeScript/Zod as source and generate JSON Schema

Author runtime validators in the SDK and derive JSON Schema/Rust types. This is
ergonomic for the first client but couples neutral events to one JavaScript
library's feature and conversion semantics. Some Zod constructs do not have one
portable JSON Schema meaning, and collector security would depend on generated
output from a client-side framework.

### Option D — Protobuf, Avro, or another binary schema system

These provide strong generation and evolution tooling, but P0 uses JSON for
inspectable SDK batches, curl/manual integration, NDJSON offline replay, reports,
and simple self-hosting. A binary protocol or schema registry adds operational
and debugging cost without a measured volume need.

### Option E — OpenAPI as the only schema source

Put event union definitions inside the REST document. Events also exist outside
one endpoint and have a lifecycle/version independent of REST resources.
Code-first OpenAPI from Rust would invert the selected language-neutral event
authority and risk two copies when fixtures or file ingestion need schemas.

### Option F — Hand-maintain Rust and TypeScript types without generation

This avoids generator quirks initially, but optionality, closed enums, decimal
strings, fingerprint envelopes, and privacy fields will drift. Tests can catch
known fixtures but cannot make duplicated definitions a source of truth.

## Decision

The canonical Landfall event wire contract shall be checked-in **JSON Schema
Draft 2020-12** under `schemas/events/v1/`. Every root schema declares:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://schemas.landfall.dev/events/v1/1.0/batch.schema.json"
}
```

The `$id` URI is a stable identifier, not a runtime network dependency. All
schema resources are bundled in the repository/server and every `$ref` resolves
through an explicit local registry. Production validation must never fetch a
schema over HTTP, follow an unregistered URI, or depend on
`schemas.landfall.dev` availability.

Direction of authority and generation:

```text
JSON Schema + version manifest + golden fixtures
        |
        +--> generated TypeScript wire DTOs
        |
        +--> verified hand-authored Rust Serde wire types
        |
        +--> embedded/compiled Rust runtime validators
        |
        +--> event-schema references used by REST documentation
```

TypeScript wire DTOs are generated and committed. Rust wire types remain
hand-authored for controlled Serde behavior and ergonomic newtypes, but they are
not allowed to broaden or redefine the contract: the collector and CLI apply the
canonical schema before typed semantic processing, and CI proves conformance
against shared fixtures. Generated types contain no domain reducer, privacy
policy, diagnosis, metric, or persistence behavior.

## Detailed design and boundaries

### Planned layout

```text
schemas/events/v1/
├── manifest.json
├── 1.0/
│   ├── batch.schema.json
│   ├── envelope.schema.json
│   ├── common/
│   │   ├── identifiers.schema.json
│   │   ├── source.schema.json
│   │   ├── fingerprint.schema.json
│   │   └── errors.schema.json
│   └── events/
│       ├── trace-created.schema.json
│       ├── blockhash-acquired.schema.json
│       ├── simulation-started.schema.json
│       ├── simulation-completed.schema.json
│       ├── signing-started.schema.json
│       ├── signing-completed.schema.json
│       ├── submission-started.schema.json
│       ├── submission-completed.schema.json
│       ├── submission-retry-scheduled.schema.json
│       ├── confirmation-wait-started.schema.json
│       ├── confirmation-wait-completed.schema.json
│       ├── status-observed.schema.json
│       ├── execution-enriched.schema.json
│       ├── business-outcome-observed.schema.json
│       └── data-quality-detected.schema.json
└── fixtures/
    ├── valid/
    ├── invalid/
    └── manifest.json
```

This is a target organization; implementation may consolidate small common
resources when it improves generator compatibility, but stable `$id` values and
the logical ownership above do not change casually.

`landfall.diagnosis.generated` and `landfall.recommendation.generated`, currently
named as events in the PRD, are not accepted producer-evidence event types in
v1.0. They are versioned derived records with evidence references under ADR-002;
feeding projector output back into its own raw input would create unnecessary
cycles and replay ambiguity. Their API/storage contracts remain typed and
versioned separately. The PRD event list will be reconciled during the Phase 0
design-document consistency update.

`manifest.json` records supported wire versions, root schema IDs, all local
resources, generator name/version/config checksum, and fixture expectations. It
contains no secrets or environment-specific values.

### Event discrimination and closed objects

An event envelope contains common fields such as `schema_version`, `event_id`,
`event_type`, timestamps, project/environment/trace/business-action identity,
source, privacy-policy version, and event-specific attributes.

The root event union uses standard JSON Schema `oneOf` branches. Each branch
constrains `event_type` with `const`, producing a discriminated union without a
non-standard `discriminator` keyword. Event-specific attributes use explicit
properties and required lists.

Objects are closed with `unevaluatedProperties: false` or an equivalent proven
composition that works under Draft 2020-12. Unknown fields are rejected. P0 has
no unregistered free-form extension object; a future extension namespace must
have its own versioned schema resource, bounds, privacy classification, and
compatibility review.

JSON Schema `default` values are documentation annotations and never mutate an
event. Producers send every semantically required value explicitly. Collector
validation does not add, coerce, trim, or silently normalize input before raw
storage.

### Primitive representation rules

- IDs use canonical lowercase UUIDv7 strings required by ADR-003. Schema pattern
  and semantic parsing both validate them.
- Timestamps use RFC 3339 UTC strings. Runtime parsing enforces supported
  precision/offset behavior; `format` annotation alone is not trusted as an
  assertion across validators.
- Durations and chain/application integers that can exceed JavaScript safe range
  use canonical unsigned/signed decimal strings with no exponent, leading plus,
  whitespace, or unnecessary leading zeros.
- Monetary/fee fields include the unit in the field name or an explicit closed
  unit enum; no floating-point SOL values enter the event contract.
- HMAC fingerprint values follow ADR-004's algorithm/key envelope and 64-character
  lowercase-hex pattern.
- Signatures, blockhashes, route labels, source names, app versions, errors, and
  all arrays/objects have explicit length/count bounds.
- Nullable data is used only when `null` has different meaning from absence;
  missing evidence is normally represented by absence plus data-quality logic.

Custom `format` checks are never the sole security constraint. Critical lexical
rules use portable patterns and are repeated by typed/semantic parsing where
necessary.

### Three validation layers

```text
1. Structural schema validation
   "Is this a bounded v1 submission.completed document?"

2. Typed semantic/security validation
   "Does this token own the environment, do fields agree, and does privacy
    policy permit them?"

3. Domain reduction
   "What does this valid evidence imply about lifecycle and diagnosis?"
```

JSON Schema owns structural fields, types, required/optional status, enum values,
numeric-string syntax, and local size/cardinality limits. It does not own:

- token authentication or project/environment authorization;
- active environment privacy policy and key-ID allowlist;
- cross-field facts such as an attempt belonging to a trace;
- signature/fingerprint agreement;
- clock trust and cross-process event order;
- whether RPC acceptance proves landing;
- diagnostic certainty or recommendations.

Those rules remain typed Rust application/domain code with explicit error and
rule versions. A structurally valid event can still be rejected by security or
semantic validation, or accepted as contradictory evidence for domain analysis.

### Collector validation path

For each bounded batch, the collector:

1. authenticates and enforces compressed/decompressed request limits;
2. parses JSON into a bounded generic representation while rejecting duplicate
   object member names before they can be collapsed;
3. selects a bundled root validator from declared schema version;
4. validates batch and every event structurally;
5. deserializes into Rust wire types;
6. performs ownership, privacy, cross-field, and canonical-value checks;
7. persists the validated semantic JSON values and projection work atomically.

Compiled validators are created once during startup and reused. Failure to load,
resolve, or compile every advertised schema keeps readiness false. The request
path never compiles schemas or performs network resolution.

An invalid event rejects the complete batch. Public errors include a stable code,
event index/ID when safely parseable, and bounded JSON Pointer/schema location;
they never echo authorization headers, endpoint URLs, signed bytes, arbitrary
payload values, or complete rejected events.

### TypeScript generation

The schema-to-TypeScript generator and configuration are pinned in the repository
lockfile. A single command such as `just generate-protocol` writes only a marked
generated directory under `packages/protocol-ts/src/generated/`.

Generated output is committed because:

- package consumers and normal builds do not need the generator;
- PR review shows the public type impact of a schema change;
- release archives remain reproducible offline;
- generator upgrades have isolated visible diffs.

Generated files have a “do not edit” header with schema/generator metadata.
Handwritten SDK builders and public conveniences wrap generated DTOs outside that
directory. CI regenerates from a clean checkout and fails on any diff.

Generated TypeScript types provide compile-time shape, not runtime trust. The SDK
constructs events through typed builders and privacy transformations; the
collector remains authoritative. A separate development/doctor validator may be
provided without forcing a full schema validator into every production SDK
bundle.

### Rust wire types and validation

`landfall-protocol` contains Serde wire DTOs, typed ID/decimal/timestamp parsers,
an embedded schema registry, and validation error categories. It contains no
Axum, SQLx, observer client, reducer, or product rules.

Rust DTOs use closed enums, `deny_unknown_fields` where composition permits,
explicit rename rules, and wrappers that preserve decimal precision. Schema
validation remains mandatory at untrusted JSON boundaries even when Serde would
accept the document, so a broader Serde behavior can never expand the public
contract.

Direct in-process events created by trusted server components use the same DTOs
and are validated in tests. Before persistence, their repository boundary still
enforces the contract and privacy policy; “internal” does not permit an observer
to write an undocumented raw event.

Generating Rust wire DTOs from JSON Schema may be reconsidered if a selected
generator produces stable ergonomic Serde types without weakening newtypes. It
would change implementation mechanics, not schema authority, and requires an ADR
amendment or generator spike record.

### Golden fixtures

Fixtures are executable protocol examples, not informal documentation. The
manifest assigns each file:

- schema version and expected event type;
- valid or invalid outcome;
- stable invalid category when applicable;
- expected normalized Rust/TypeScript round-trip representation;
- privacy classification and prohibited-value expectation.

The valid corpus includes each event type plus complete success, rejection,
timeout/later success, expiry, execution failure, identical retry, replacement,
observer disagreement, and missing-evidence sequences. Invalid fixtures cover
unknown fields/types/events, wrong UUID versions, unsafe integers, malformed
timestamps/fingerprints, oversized/deep values, forbidden secrets, and mode
violations.

Schema validator, Rust parser/serializer, TypeScript compiler/runtime fixture
harness, offline CLI, and ingestion contract tests consume the same files.

### Version and compatibility policy

Each event carries `schema_version` as canonical `MAJOR.MINOR`, initially `1.0`.
The major directory groups compatible protocol evolution; the exact declared
minor selects a registered schema resource. Raw events retain their original
version forever and are never relabeled in place.

Compatible minor changes may:

- add an optional bounded field with one meaning and unit;
- add a new event type when collector capability negotiation is explicit;
- add a separately versioned registered extension;
- tighten documentation without changing accepted instances.

Breaking changes require a new major when they:

- remove or rename a field/event type;
- make an optional field required;
- change type, unit, meaning, privacy classification, or fingerprint input;
- add/change a closed enum value in a way that breaks exhaustive consumers;
- reinterpret absence, `null`, zero, error, or timestamp semantics;
- newly reject previously valid persisted wire data for structural reasons.

Security/privacy policy can reject data independently of schema compatibility,
but the rejection must use a versioned policy/error rather than pretending the
old document never matched its schema.

Collectors advertise an explicit supported version/event-type matrix. Unknown
major/minor versions or event types are rejected with an actionable compatibility
code. SDK `doctor` checks compatibility before production use. No producer assumes
that “same major” means an older collector already knows a newly added event.

An explicit pure upcaster may translate a retained older event into a newer
in-memory DTO for reduction. It records source/target versions, is deterministic,
has golden fixtures, and never rewrites the raw event. Silent best-effort field
coercion is prohibited.

### Relationship to OpenAPI

ADR-008 defines code-first OpenAPI for REST resources. The batch-ingestion route
must reference or embed a generated snapshot of this canonical event schema; it
must not independently redefine event fields in Utoipa annotations. OpenAPI can
describe HTTP headers, status codes, and response envelopes while JSON Schema
remains authoritative for each event document.

This ADR does not select the exact TypeScript generator version, public npm
package publishing workflow, Rust schema-validator version, or REST OpenAPI
composition tool. Those are pinned and tested during workspace/protocol
implementation without changing the authority direction.

## Consequences

### Positive

- One language-neutral contract governs SDK, collector, CLI, fixtures, and future
  adapters.
- TypeScript producers receive generated discriminated unions and exact field
  optionality without manually mirroring Rust.
- Runtime schema validation makes the checked-in contract an actual enforcement
  boundary rather than documentation.
- Strict closed objects and bounds reduce secret leakage, accidental cardinality,
  and parser/database abuse.
- Retained events remain interpretable through explicit version/upcaster rules.
- Committed generated output and drift checks make protocol changes visible in
  code review.
- Domain rules remain clean typed code rather than being forced into schema
  expressions or generators.

### Negative

- Rust DTOs and JSON Schema are two representations that require fixtures and
  runtime validation to prevent drift.
- Collector processing parses/validates structurally before typed semantic work.
- Generator limitations can influence schema composition and generated type
  readability.
- Every schema evolution requires manifests, fixtures, generated diffs, and
  compatibility review.
- Closed enums/objects reduce ad hoc extensibility and require deliberate version
  changes.
- Supporting multiple retained minor/major versions increases validator and
  upcaster maintenance.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Generated TypeScript drifts from schema | SDK emits incompatible events | Pinned generator/config, committed output, clean-regeneration CI, shared fixtures |
| Rust DTO accepts/rejects differently | Collector contract differs from SDK/docs | Mandatory schema-first validation, Serde conformance fixtures, property/fuzz tests |
| Remote/cyclic `$ref` causes SSRF or resource exhaustion | Startup/request availability or network access | Bundled allowlisted registry only, no runtime fetch, compile once, bounded/nonrecursive event resources |
| Unbounded string/array/object reaches immutable storage | Memory, disk, index, or privacy abuse | Explicit max lengths/items/properties/depth, request body limits, semantic cardinality controls |
| Regex/format behavior differs across validators | Cross-language acceptance drift | Portable simple patterns, explicit format configuration, typed parse, cross-validator invalid fixtures |
| Optional addition breaks older collector | Events are dropped during rollout | Capability/doctor negotiation, exact minor version, staged collector-before-SDK rollout |
| Schema expresses business rules incompletely | Structurally valid event produces unsafe conclusion | Separate semantic/privacy validation and deterministic domain rules; contradictory evidence remains visible |
| Validation error reflects sensitive input | Secret leakage through response/log | Stable category and bounded pointer only; no value echo; redaction tests |
| Generator upgrade creates noisy/unreviewed contract changes | Accidental public API break | Upgrade in isolated commit, snapshot diff review, semver analysis, fixture rerun |

## Security and privacy impact

Schemas implement a positive allowlist: only named bounded properties exist, and
unknown nested content is rejected. This is stronger than accepting arbitrary
JSON and scanning it for secret patterns afterward. Secret-pattern checks remain
defense in depth.

The schema registry is part of the trusted application artifact. External `$ref`
loading is disabled, preventing schema URLs from becoming SSRF, supply-chain, or
availability inputs. CI validates every schema against the selected meta-schema
and ensures all references resolve from committed resources.

Schema validity does not authorize a field under the current privacy mode. The
collector repeats ADR-004 policy enforcement after structural parsing and before
persistence. Invalid errors and diagnostics use paths/categories without
including rejected values.

No schema `default`, coercion, or normalizer may insert sensitive data or turn an
invalid representation into a valid event silently. Raw storage contains the
validated parsed JSON value, not the original whitespace/property order or other
request-byte formatting; audits reproduce the accepted semantic document rather
than claim byte-for-byte request preservation.

## Reliability and failure behavior

Schema resources and compiled validators are loaded before readiness. A missing,
duplicate-ID, unresolved-reference, unsupported-dialect, or compilation error
prevents ingestion readiness rather than running with partial validation.

An invalid event rejects its whole batch before database writes. A duplicate but
valid event remains idempotent under its event ID. Database failure occurs only
after structural/semantic validation and still returns no false `202 Accepted`.

SDK/collector version mismatch produces a stable compatibility error and health
status. SDK telemetry remains fail-open for the customer's Solana operation, but
events rejected for version mismatch count as observable telemetry loss and are
not retried indefinitely as transient failures.

Old retained events remain bound to their original schema version. If an
upcaster/reducer fails for one trace, its projection job becomes visibly failed
without blocking unrelated traces or mutating the source event.

## Performance and capacity impact

The collector parses at most the configured one-MiB decompressed batch and 100
events, then validates them against precompiled in-memory schemas. No schema I/O,
network lookup, code generation, or compilation occurs per request.

Structural validation cost must be included in the 100-event batch p95 target of
250 ms and collector burst target of 500 events/second. Benchmarks include valid
worst-sized events, early/late invalid fields, `oneOf` dispatch, deep-but-allowed
structures, and privacy/semantic validation.

Closed typed fields reduce downstream JSONB indexing and high-cardinality costs.
Generated TypeScript has no runtime cost unless an optional development validator
is explicitly included.

## Operational impact

Build/release metadata exposes supported event versions, schema bundle checksum,
generator version, and rule-set versions. `doctor` compares SDK and collector
capabilities and validates a disposable fixture without contaminating product
metrics.

Protocol changes use one generation command and a review checklist covering
compatibility, units, privacy, bounds, fixtures, generated output, upcasters,
docs, and release notes. CI rejects dirty generation, invalid meta-schemas,
unresolved refs, fixture disagreement, and schema checksum drift.

Operators do not download schemas at runtime. Release archives/container images
include the exact schema bundle used by the server, and reports identify source
schema versions without embedding the entire bundle.

## Verification

- Every schema validates against the Draft 2020-12 meta-schema and has a unique,
  registered `$id`; every `$ref` resolves locally with network access disabled.
- Clean generation produces no Git diff; generator/config changes produce an
  intentional reviewed diff and metadata update.
- Every valid fixture passes schema, Rust, TypeScript, CLI, and ingestion paths;
  every invalid fixture fails with the expected stable category.
- Rust and TypeScript round trips preserve canonical IDs, decimal strings,
  timestamps, fingerprint envelopes, absent/null distinction, and enum values.
- Property/fuzz tests cover unknown fields, type confusion, large/deep values,
  numeric precision, duplicate object keys at parser boundary, and event-union
  discrimination.
- Privacy fixtures prove schemas/builders cannot carry raw signed transactions,
  signer secrets, credentials, endpoint URLs, arbitrary environment/request
  objects, or unbounded logs.
- Compatibility fixtures prove supported minor additions, unknown versions/types,
  explicit upcasting, and collector-before-SDK rollout behavior.
- Error snapshot tests prove responses/logs contain bounded pointers/categories
  but never rejected sensitive values.
- Benchmarks prove schema plus semantic validation meets ingestion latency and
  throughput targets at configured limits.

## Rollout and migration

This is the initial event contract. Phase 2 creates the v1.0 schemas and fixture
manifest first, then TypeScript generation, Rust DTOs/validators, SDK builders,
and collector integration. No public producer is released before the cross-
language corpus passes.

Generator and validator tools are pinned after small compatibility spikes. Their
selection may change before first release if they cannot support the accepted
Draft 2020-12 composition, but the schema-first authority and output contracts do
not change.

After release, compatible schema rollout is collector/CLI support first, then SDK
production. Breaking v2 runs beside v1 for a documented migration/retention
period; raw v1 events remain unchanged and use versioned reducers/upcasters.

## Reconsideration triggers

Revisit this approach if:

- the selected generator or Rust validator cannot implement required Draft
  2020-12 semantics consistently despite schema simplification;
- maintaining handwritten Rust DTO conformance causes repeated production drift;
- a non-JSON transport becomes necessary for measured throughput or ecosystem
  integration;
- browser/other-language producers require a different schema distribution or
  runtime validation strategy;
- many concurrently supported major versions make per-event validation/replay
  operationally unacceptable;
- JSON parsing/schema validation materially prevents capacity targets after
  profiling and bounded optimization;
- an industry interoperability contract supersedes Landfall-specific events.

Any replacement must retain language neutrality, explicit privacy/bounds,
versioned raw-event interpretation, and shared valid/invalid fixtures.

## References

- [JSON Schema Draft 2020-12 core specification](https://json-schema.org/draft/2020-12/json-schema-core)
- [JSON Schema Draft 2020-12 validation specification](https://json-schema.org/draft/2020-12/json-schema-validation)
- [JSON Schema Draft 2020-12 release notes](https://json-schema.org/draft/2020-12/release-notes)
- [Product requirements: event schema](../product-requirements-document.md#17-event-schema)
- [Product requirements: collector and ingestion](../product-requirements-document.md#134-collector-and-ingestion)
- [Product requirements: testing strategy](../product-requirements-document.md#23-testing-strategy)
- [System design: batch event ingestion](../system-design.md#64-batch-event-ingestion)
- [System design: SDK privacy pipeline](../system-design.md#91-typescript-sdk)
- [Technical implementation plan: contracts and code generation](../technical-implementation-plan.md#35-contracts-and-code-generation)
- [Technical implementation plan: versioned event protocol](../technical-implementation-plan.md#9-phase-2--versioned-event-protocol)
