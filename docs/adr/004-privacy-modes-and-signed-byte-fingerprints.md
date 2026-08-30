# ADR-004: Privacy modes and signed-byte fingerprints

- **Status:** Accepted
- **Date:** 2026-08-30
- **Decision owners:** Landfall maintainers
- **Related requirements:** G8, FR-SDK-004, FR-SDK-006, FR-PRIV-001 through FR-PRIV-003, security/privacy requirements, retention, sanitized reports
- **Related ADRs:** [ADR-001](001-modular-monolith-and-two-container-topology.md), [ADR-002](002-immutable-event-inputs-and-relational-projections.md), [ADR-003](003-business-action-trace-attempt-event-and-alias-identifiers.md)
- **Supersedes:** None
- **Superseded by:** None

## Context

Landfall needs to recognize repeated submission of the same signed Solana
transaction without retaining the signed transaction itself. It also needs enough
evidence to observe on-chain status and diagnose configuration problems, while
serving teams for whom public signatures, addresses, program IDs, timing, and
transaction strategy become sensitive when collected together.

A raw Solana transaction contains the message, account keys, instructions, and
signatures. Even though much of a landed transaction may become public on-chain,
retaining raw bytes centrally also captures transactions that never landed and
creates an unnecessary payload, strategy, and incident-response liability.
Landfall's core product value does not require custody, signing, or replaying
customer transactions.

An unkeyed SHA-256 digest would distinguish replacements from identical-byte
retries, but it is globally stable: anyone who obtains candidate transaction
bytes can calculate the same value and correlate databases, reports, or
deployments. A keyed fingerprint can preserve equality within one customer
environment while limiting that cross-deployment correlation.

The privacy contract must cover more than the fingerprint algorithm. SDK,
collector, observer, database, logs, APIs, dashboard, reports, deletion, backups,
and key rotation must agree on which fields are permitted. A mode name without a
field-level and failure contract would create false confidence.

## Decision drivers

- Identical exact signed bytes must correlate within an environment so retries do
  not inflate transaction counts.
- Any signed-byte change must produce a different fingerprint and replacement
  trace.
- Raw signed bytes and signing secrets are unnecessary for P0 product value and
  must not become a telemetry dependency.
- A database/report leak should not expose a globally reusable transaction digest
  when an environment-scoped alternative is practical.
- Standard mode must support central signature observation and retrospective
  enrichment.
- Full diagnostic collection must be explicit and allowlisted rather than a
  general “store more JSON” switch.
- Strict mode must support useful transaction correlation while keeping signature
  and public account/program identity inside the customer boundary.
- SDK and collector enforcement must be independently testable; the collector
  cannot trust a client-declared mode without validating fields.
- Fingerprint output, algorithm, input serialization, purpose, and key version
  must be unambiguous across Rust and TypeScript.
- Key rotation must not silently turn retries into replacements.
- Export and deletion must include derived/searchable copies, not only raw events.

## Options considered

### Option A — Environment-keyed HMAC-SHA-256 fingerprint and explicit modes

Compute a full HMAC-SHA-256 over exact serialized signed transaction bytes in the
customer process, using a dedicated random environment key and domain-separated
input. Send only the fingerprint envelope. Store signatures and other public
fields according to the environment's privacy mode.

This preserves deterministic equality within a key scope, prevents equality
comparison across environments without the relevant key, and avoids transmitting
raw bytes to the collector. It requires secret provisioning, key/version metadata,
rotation handling, and health diagnostics.

### Option B — Plain SHA-256 of signed bytes

Hash the exact bytes without a key. This is operationally simple and permits
correlation across SDK instances automatically. It also creates a globally stable
identifier: a party with public/candidate transaction bytes can perform offline
matching against stolen databases or reports. Changing to a keyed construction
later would complicate historical identity.

Landfall does not support plain SHA-256 transaction fingerprints in P0, including
standard mode. A future interoperability use case requires a new ADR and explicit
privacy warning.

### Option C — Signature-only correlation

Use the Solana transaction signature instead of a fingerprint. Signature lookup
is useful in standard/full modes, but a signature may be unavailable after a
client timeout, cannot represent pre-signing stages, and is intentionally absent
from strict central storage. It also cannot independently detect adapter bugs
that associate an unexpected signature with bytes.

Signature remains a correlation and observation key where policy permits, not
the only retry/replacement identity.

### Option D — SDK trace ID only, no content fingerprint

Trust propagation of `TraceId` and never correlate by signed content. This avoids
a pseudonymous content value and secret key, but separate processes can assign
different trace IDs to the same bytes. Retry/replacement metrics and alias
recovery would become dependent on perfect instrumentation.

### Option E — Send raw signed bytes and hash on the server

Central calculation ensures one implementation and hides the HMAC key from the
SDK. It defeats the primary privacy boundary: bytes for landed and non-landed
transactions leave the signing application, enter transport/logging/parser
surfaces, and may reach immutable storage or backups after one bug.

### Option F — Store raw signed bytes encrypted

Encryption at rest reduces disk disclosure but the server still receives and can
decrypt the payload. It adds encryption keys, nonce/lifecycle design, backup
recovery, privileged access, and a future temptation to rebroadcast transactions.
P0 has no requirement that justifies this risk.

## Decision

### Fingerprint algorithm

P0 transaction fingerprints shall use this versioned contract:

```text
algorithm_id = "lf-hmac-sha256-v1"
domain       = UTF8("landfall/solana/signed-transaction/v1") || 0x00
input        = exact serialized signed transaction bytes
fingerprint  = HMAC-SHA-256(environment_key, domain || input)
wire_value   = lowercase hexadecimal encoding of all 32 output bytes
```

The environment key shall contain 32 bytes (256 bits) generated by a
cryptographically secure random number generator. The HMAC output is not
truncated. Passwords, ingestion/admin tokens, RPC credentials, signatures,
project IDs, and customer identifiers must not be reused as fingerprint keys.

The input is the exact byte sequence passed to the supported submission client
after all required signatures are present. It is not base58/base64 text, JSON,
an object reserialization, message-only bytes, or an unsigned transaction. An
adapter that cannot obtain this byte sequence emits no fingerprint and reports a
data-quality limitation; it must not silently use a different input.

The event carries a structured fingerprint envelope:

```json
{
  "algorithm": "lf-hmac-sha256-v1",
  "key_id": "0198f0c1-1234-7abc-8def-1234567890ab",
  "value_hex": "64-lowercase-hex-characters"
}
```

`key_id` is a non-secret UUIDv7 identifier for one environment key version. The
database stores the 32-byte value as `BYTEA` plus algorithm/key IDs; it does not
store only an unlabeled `signed_bytes_digest`. Equality and uniqueness are scoped
by `(environment_id, algorithm_id, key_id, fingerprint)`.

### Privacy modes

Landfall defines three ordered but non-interchangeable environment modes:

1. `standard` — P0 default; central signature observation with minimized
   structured telemetry;
2. `full` — P0 explicit opt-in; standard data plus specifically allowlisted
   public diagnostic identifiers;
3. `strict` — P1; signature/account/program identity and observer RPC remain in
   the customer boundary, while pseudonymous structured results are sent.

Mode is configured and versioned per environment. An event records the applied
privacy-policy version. The collector enforces the server-side environment
policy and rejects an event atomically when it contains a field more permissive
than allowed. Missing fields are accepted when structurally valid but lower data
quality; the server does not invent them.

Raw signed transaction storage is unsupported in P0 in every mode. There is no
hidden debug flag, custom metadata escape hatch, report option, or logging path
that may persist the bytes. A future opt-in raw-transaction feature requires a
new threat model and superseding ADR.

## Detailed design and boundaries

### Field matrix

| Field or capability | Standard P0 | Full P0 | Strict P1 |
|---|---|---|---|
| Landfall event/trace/attempt IDs | Stored | Stored | Stored |
| HMAC signed-byte fingerprint | Stored when bytes available | Stored when bytes available | Stored when bytes available |
| Solana signature | Stored | Stored | Remains local |
| Signature lookup API | Enabled with log scrubbing | Enabled with log scrubbing | Disabled |
| Central signature polling/getTransaction | Enabled | Enabled | Disabled; local SDK/sidecar observes |
| Raw recent blockhash | Stored | Stored | Remains local |
| `lastValidBlockHeight` | Stored | Stored | Stored |
| Route ID/display label | Stored | Stored | Stored |
| Credential-bearing route URL | Never stored | Never stored | Never stored |
| Fee/compute/retry configuration | Bounded structured fields | Bounded structured fields | Bounded structured fields |
| Public account addresses | Not stored by default | Only explicit environment allowlist | Remain local |
| Program/instruction identifiers | Not stored by default | Only explicit environment allowlist | Remain local |
| Full logs or arbitrary error objects | Never stored | Never stored | Never stored |
| Bounded normalized error/result | Stored | Stored | Stored |
| Raw signed transaction bytes | Unsupported | Unsupported | Unsupported centrally |

Public on-chain data is not automatically low-risk. Central correlation of
addresses, programs, timing, routes, failures, and app versions can reveal
customer strategy, so full mode remains an explicit persistent UI/configuration
state rather than a per-call convenience flag.

### SDK privacy pipeline

The SDK/adapter processes a signed transaction in this order:

```text
exact signed bytes held by customer code
        |
        +-- calculate HMAC fingerprint in memory
        |
        +-- discard Landfall's byte reference as soon as calculation ends
        |
        +-- select only typed allowlisted lifecycle fields
        |
        +-- transform/remove signature, blockhash, account, and program fields
        |   according to the environment policy
        |
        +-- redact endpoint credentials and bounded text
        |
        +-- serialize event; raw bytes never enter event attributes
```

The SDK API accepts a supported transaction/serialized-byte view only at the
fingerprinting boundary. No public custom-metadata API accepts arbitrary
transaction objects, signer/keypair objects, headers, environment variables, or
request bodies. The SDK must not log function arguments on fingerprint failure.

JavaScript cannot guarantee immediate memory erasure. Implementations avoid
unnecessary copies, keep byte references short-lived, and zeroize owned temporary
buffers where safe, but documentation must not claim protection from a compromised
customer process or memory dump.

### Collector enforcement

The collector treats the SDK as untrusted. Before immutable persistence it:

- authorizes project/environment ownership;
- resolves the active privacy-policy version;
- validates the fingerprint algorithm, key ID, full 32-byte lowercase-hex value,
  and maximum count;
- rejects raw-byte-like fields and prohibited key names;
- rejects signature/blockhash/account/program fields forbidden by the mode;
- re-redacts endpoint/error text;
- records redaction/rejection categories without recording rejected values.

Pattern-based secret detection is defense in depth. Typed allowlists and the
absence of arbitrary nested objects are the primary controls. The collector
cannot cryptographically verify a fingerprint without the original bytes; an
environment ingestion token is therefore trusted to submit telemetry evidence,
not to affect customer funds or Solana execution.

### Key provisioning and isolation

`landfall init` or an equivalent local administrative command generates the
environment key and non-secret `key_id`. The secret is provisioned to each SDK or
strict-mode sidecar that must produce comparable fingerprints through an
environment variable, mounted secret, or platform secret manager. It is never
written to a committed config file, event, telemetry table, URL, log, report, or
dashboard response.

The central server needs the configured algorithm/key ID allowlist but does not
need the fingerprint secret for normal ingestion, equality lookup, aliasing, or
projection. Keeping the key in the customer instrumentation boundary reduces the
impact of a database or dashboard compromise.

Environment initialization stores a non-secret key verifier in control state:

```text
key_check_domain = UTF8("landfall/fingerprint-key-check/v1") || 0x00
key_check_input  = canonical 16-byte EnvironmentId
key_check_value  = HMAC-SHA-256(environment_key,
                               key_check_domain || key_check_input)
```

An SDK `doctor` operation computes this value locally and sends it through the
disposable health-check path; the server compares it with the registered value
for `key_id` and does not persist the request as production telemetry. The
verifier is never accepted as a transaction fingerprint. It lets multiple SDK
instances detect same-ID/different-key configuration without giving the server
the secret; its security still depends on the required random 256-bit key.

Every environment uses an independently generated key. Development, staging,
mainnet production, and separate customer deployments therefore produce
different fingerprints for identical bytes. Sharing one key across environments
is rejected by operational guidance and detectable configuration fingerprints;
Landfall does not send those fingerprints to a hosted registry.

The same environment root key may be used only through explicit input-domain
labels. External business-correlation pseudonyms use a different domain such as
`landfall/business-correlation/v1`, never the signed-transaction domain. A future
need for stronger key separation may derive independent subkeys, but it must
preserve versioned purpose labels and test vectors.

### Rotation and key epochs

Each stored fingerprint includes `key_id` because HMAC values generated under
different keys are intentionally incomparable. Routine planned rotation uses a
bounded overlap:

1. configure a new current key and retain at most one previous key;
2. during the grace period, SDKs emit fingerprints under both keys for the same
   signed bytes, with the new key marked primary;
3. the server associates both fingerprint records with the same trace and can
   bridge aliases across key epochs without raw bytes;
4. wait longer than SDK buffer/retry and active transaction observation windows;
5. remove the previous key from SDK configuration while retaining its non-secret
   ID and stored fingerprint records.

An event carries at most two transaction fingerprints, preventing an unbounded
key/history list. Database design therefore uses a normalized
`trace_fingerprints` relationship rather than a single digest column.

Emergency rotation after suspected key compromise skips dual emission with the
compromised key. Correlation across the cutover is then available only through
other permitted evidence, such as signature in standard/full mode. Strict mode
may lose automatic cross-epoch retry correlation; the system exposes this as a
data-quality boundary rather than weakening the emergency response.

The fingerprint key is not automatically rotated with ingestion-token revocation.
They have different purposes and blast radii. Key destruction is a crypto-erasure
aid for offline correlation but does not delete already stored signatures,
metadata, fingerprints, projections, reports, or backups.

### Strict mode boundary

Strict mode is a P1 deployment profile, not a P0 promise. A local SDK/sidecar
retains the signature, recent blockhash, account/program identity, and observer
RPC credentials. It performs status polling and transaction enrichment locally,
then emits allowlisted pseudonymous status, commitment, execution, fee/compute,
and data-quality events plus the environment-keyed fingerprint.

The central server cannot perform signature search, retrospective `getTransaction`,
or independent observer verification in strict mode. Diagnoses and reports must
label this evidence source and limitation. If signed bytes are unavailable and
no fingerprint can be emitted, Landfall accepts the trace ID but reports that
server-side retry alias recovery is unavailable.

### Mode changes

Changing an environment from full/standard to a stricter mode affects new data
only until an explicit cleanup job removes or reprojects previously retained
fields. The UI/CLI must show this and offer a dry-run inventory before deletion.
Backups remain subject to their own expiry process.

Changing to a more permissive mode requires explicit administrative confirmation
and a new policy version. It cannot reconstruct fields omitted from old events.
SDKs using an older, more permissive policy are rejected; SDKs sending fewer
fields remain accepted with coverage warnings.

### Query, logs, and export

Signature lookup in standard/full mode removes the signature path from access
logs and stores no dashboard search history. Raw fingerprint values are not
normal dashboard search facets or report identifiers; users work with Landfall
trace IDs.

Every report/export runs a second redaction pass. Its output profile can be more
restrictive than the environment but never more permissive. The default
shareable profile removes signatures, raw blockhashes, accounts/programs, and
environment fingerprints, replacing trace relationships with report-local
opaque IDs where needed. Export metadata records the source privacy-policy and
export-redaction versions without embedding secret keys.

This ADR does not define exact event JSON Schema syntax, secret-manager product,
Rust/TypeScript crypto-library versions, or report template. It fixes the
cryptographic input/output contract, privacy field boundaries, and key lifecycle
those implementations must preserve.

## Consequences

### Positive

- Retry/replacement correlation works without retaining or transmitting raw
  signed transaction bytes.
- Database/report fingerprints cannot be directly matched across independently
  keyed environments.
- One default fingerprint construction applies consistently to standard, full,
  and strict modes.
- Strict mode can correlate retries centrally without revealing the signature or
  enabling central chain queries.
- Algorithm/key labels prevent accidental comparison of incompatible values.
- Full mode increases diagnostic evidence only through explicit field allowlists.
- Report-local pseudonyms reduce leakage when sanitized artifacts are shared.

### Negative

- Every instrumented application needs an additional environment secret and key
  ID in addition to its ingestion token.
- Misconfigured keys split identical transactions into different correlation
  epochs and reduce alias quality.
- Planned rotation requires bounded dual fingerprints and a normalized table.
- The server cannot verify a claimed HMAC without receiving the prohibited raw
  bytes.
- HMAC output remains linkable within one environment and therefore remains
  sensitive telemetry, not anonymous data.
- Standard/full signatures remain globally public correlation values by design.
- Full diagnostic mode still cannot provide arbitrary logs or raw transaction
  inspection.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Raw signed bytes enter immutable telemetry/logs | Sensitive non-landed transaction content persists | No raw-byte field, typed SDK boundary, collector prohibited-field checks, secret fixtures, log/report scans, P0 feature exclusion |
| Fingerprint key is committed or logged | Offline matching and alias poisoning become easier | Secret-manager/env provisioning, redacted config display, secret scanning, separate key ID, incident rotation runbook |
| Different SDK instances use different keys | Same signed bytes become separate traces | Environment doctor test vector, key-ID allowlist, configuration health metric, alias via signature where allowed |
| Routine rotation breaks in-flight correlation | Retry counted as replacement | At-most-two dual fingerprints during grace period; wait for buffers/validity windows; bridge aliases transactionally |
| Compromised ingestion token submits forged fingerprint | Unrelated traces may be merged | Environment scope, signature/fingerprint conflict detection, auditable aliases, token revocation; accept residual strict-mode telemetry-integrity risk |
| “HMAC” is presented as anonymity | Customer underestimates timing/volume/linkability | UI/docs call it environment-scoped pseudonymization; disclose key-compromise/offline-guessing limits |
| Full mode becomes arbitrary payload storage | Secrets, strategy, and unbounded cardinality enter database | Explicit per-field allowlist, hard sizes/cardinality, no arbitrary objects/logs, persistent mode warning |
| Mode downgrade leaves old permissive data | Operator believes data is already removed | Inventory/dry-run, explicit cleanup job, policy-version display, backup limitation warning |
| Export includes a stable environment fingerprint | Shared reports become correlatable | Default shareable profile removes/re-pseudonymizes it; second redaction pass and snapshot tests |

## Security and privacy impact

HMAC provides keyed deterministic pseudonymization, not encryption, anonymity,
authorization, proof of transaction ownership, or proof that a transaction was
submitted. If the environment key and candidate signed bytes are both available,
an attacker can recompute fingerprints. Key compromise therefore requires
rotation and incident assessment.

The 256-bit key is generated randomly and protected separately from telemetry.
HMAC comparison in application code uses established libraries and constant-time
operations where applicable; database equality indexes are still used for normal
lookup because the fingerprint value itself is a stored pseudonymous index.
Custom cryptographic implementations are prohibited.

Signed bytes are handled only transiently inside the already trusted customer
process. Landfall never requests private keys, seed phrases, signer callbacks, or
custody. The adapter fingerprints only after signing and before discarding its
temporary reference; transaction submission continues directly to the customer
route.

Addresses and signatures are public on-chain but can be commercially sensitive
when joined with private flow labels, routes, timing, app versions, and failures.
Privacy classification therefore depends on collected context, not merely
whether an individual field is public.

## Reliability and failure behavior

Fingerprinting is local and synchronous over an already available small byte
buffer; it must not add a Landfall network call to transaction submission. A
fingerprint error records SDK health/data-quality state and, by default, does not
fail or alter the customer's signing or submission operation.

The SDK assigns the same fingerprint envelope to all submission attempts for the
same bytes and preserves it across telemetry retries. Recomputing the HMAC under
the same key is deterministic. A replacement byte sequence receives a different
value with overwhelming probability and a separate trace under ADR-003.

Collector mode/key validation occurs before database commit. A batch containing
forbidden fields, malformed fingerprint data, unknown key IDs, or a policy
downgrade is rejected atomically with a stable non-sensitive error code. Rejected
values are never repeated in logs or responses.

If the key is missing or unreadable, the SDK continues the customer transaction,
does not fall back to plain SHA-256, and reports fingerprint coverage loss. If
different key epochs appear without an overlap bridge, Landfall does not assume
inequality means replacement; it marks correlation incomplete and uses permitted
signature evidence where available.

## Performance and capacity impact

Each signed transaction requires one HMAC-SHA-256 over the exact serialized byte
sequence, or two during a bounded rotation grace period. This is negligible
relative to signing, network submission, and RPC observation but shall still be
measured in SDK overhead benchmarks.

Each fingerprint stores 32 value bytes plus algorithm/key IDs and index overhead.
At approximately 50,000 traces/day, the normalized table is small relative to
raw events; routine dual-key rotation temporarily adds one row per affected
trace. A unique index on environment, algorithm, key ID, and value supports
canonical trace lookup.

Fingerprinting must remain inside the SDK synchronous-overhead p95 budget of
five milliseconds, excluding the customer's original transaction work. No
fingerprint operation may allocate unbounded buffers or serialize arbitrary
objects.

## Operational impact

Environment initialization generates the key and key ID, shows where to place
the secret, and never prints it again by default. `doctor` verifies algorithm,
known key ID, policy version, and the domain-separated environment `key_check`
across SDK/collector configuration without accepting production signed bytes.

Operational status exposes fingerprint coverage, unknown key IDs, mode mismatch,
redaction/rejection counts, key-epoch overlap, and rotation age without exposing
key material or raw fingerprint values. Logs use trace/event IDs for correlation.

Runbooks are required for routine rotation, emergency compromise, privacy-mode
change, live-data deletion, report revocation, and backup expiry. Removing a key
does not substitute for data deletion, and deleting database rows does not
immediately erase external backups.

## Verification

- Rust and TypeScript produce the same full 32-byte HMAC-SHA-256 value for fixed
  legacy and v0 signed-byte fixtures, domain label, key, and key ID.
- Identical bytes under one key produce identical values; a one-bit byte change,
  different environment key, different domain, or different algorithm version
  produces a different value.
- RFC-compatible HMAC test vectors validate library configuration; no custom HMAC
  implementation is introduced.
- SDK-to-database secret fixtures prove raw signed bytes, private keys, seed
  phrases, credentials, and prohibited objects never enter events, projections,
  logs, errors, reports, or snapshots.
- A field-matrix test accepts every allowed standard/full/strict fixture and
  rejects every more-permissive field atomically at the collector.
- Missing key tests prove no plain-hash fallback and no customer transaction
  failure, while fingerprint coverage decreases visibly.
- Two SDK instances using the same key correlate; different keys produce a
  `key_check` doctor/data-quality warning rather than an unsafe merge.
- Planned rotation fixtures emit at most two fingerprints and preserve one trace
  across the key epoch; emergency rotation documents expected strict-mode loss.
- Export snapshots prove the default shareable profile removes environment-stable
  signature/fingerprint/account/program/blockhash identifiers.
- Mode-change and deletion tests remove prohibited live raw/derived/search/report
  data and accurately report backup limitations.

## Rollout and migration

This is the initial fingerprint/privacy implementation, so no production data
migration exists. Implementation starts with policy enums and field
classification, cross-language HMAC fixtures, key provisioning/doctor support,
SDK transformation, collector enforcement, normalized trace fingerprints, and
export redaction.

The current system-design placeholder `signed_bytes_digest` is replaced during
DDL implementation by versioned fingerprint records containing algorithm, key
ID, and value. No unlabeled production digest column is introduced.

Standard is the default P0 environment mode. Full requires explicit enablement
and field allowlists. Strict is recognized as a future P1 profile and must not be
advertised as operational until its local observer/sidecar and limitation tests
pass.

Changing algorithm, domain label, input serialization, output encoding, or key
scope creates a new algorithm version. It never reinterprets stored values in
place. A migration uses overlapping fingerprint records and preserves trace
aliases/evidence under ADR-003.

## Reconsideration triggers

Revisit this decision if:

- a validated customer requires raw transaction retention and accepts a new
  threat model, access controls, encryption, audit, and deletion design;
- strict mode customers reject even environment-scoped central fingerprints;
- supported clients cannot expose one stable exact signed serialization;
- hardware-backed pseudonymization or managed KMS becomes a paid requirement;
- HMAC-SHA-256 guidance or available runtime support materially changes;
- key provisioning/rotation causes unacceptable adoption or correlation failure;
- hosted multi-tenancy requires tenant-specific key custody, regional key
  management, or separation from customer instrumentation;
- a cryptographic or implementation weakness affects the chosen construction.

Any replacement must preserve retry/replacement metric semantics and explicitly
define how old/new fingerprint epochs correlate without raw signed bytes.

## References

- [RFC 2104: HMAC—Keyed-Hashing for Message Authentication](https://www.rfc-editor.org/rfc/rfc2104.html)
- [RFC 7518: HMAC with SHA-256 key-size requirement](https://www.rfc-editor.org/rfc/rfc7518.html#section-3.2)
- [Product requirements: transaction fingerprint](../product-requirements-document.md#fr-sdk-004--transaction-fingerprint-p0)
- [Product requirements: privacy modes](../product-requirements-document.md#133-privacy-modes)
- [Product requirements: security and privacy](../product-requirements-document.md#20-security-and-privacy-requirements)
- [System design: security and privacy](../system-design.md#45-security-and-privacy)
- [System design: SDK privacy pipeline](../system-design.md#91-typescript-sdk)
- [Technical implementation plan: TypeScript SDK](../technical-implementation-plan.md#15-phase-8--typescript-sdk)
