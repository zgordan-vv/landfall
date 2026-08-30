# ADR-003: Business-action, trace, attempt, event, and alias identifiers

- **Status:** Accepted
- **Date:** 2026-08-30
- **Decision owners:** Landfall maintainers
- **Related requirements:** FR-SDK-001, FR-SDK-003, FR-SDK-004, FR-INGEST-003, FR-ENGINE-003, trace search, metric definitions, lifecycle data model
- **Related ADRs:** [ADR-001](001-modular-monolith-and-two-container-topology.md), [ADR-002](002-immutable-event-inputs-and-relational-projections.md); ADR-004 and ADR-005 (planned)
- **Supersedes:** None
- **Superseded by:** None

## Context

One customer intention can create several distinct Solana transactions, and one
signed transaction can be submitted several times. Landfall must distinguish
these cases because they have different reliability, cost, safety, and metric
meaning:

- a user requests one swap, payout, or position update;
- the application builds and signs one transaction;
- the same signed bytes are submitted to one or several routes, perhaps after a
  timeout;
- the application may rebuild and sign a replacement with a new blockhash, fee,
  instruction, or signature;
- more than one replacement may land or even execute successfully.

Counting each submission call as a transaction inflates landing denominators and
cost. Treating every replacement as the same transaction hides possible duplicate
business effects. Using only a Solana signature as identity fails before signing,
under strict privacy, and when application evidence arrives without a returned
signature. Using only an SDK-generated trace ID fails when separate processes
assign different IDs to the same signed bytes.

The identity model must therefore separate customer intent, signed transaction,
transport attempt, and evidence occurrence while supporting safe canonicalization
when high-confidence correlation arrives late.

## Decision drivers

- Identical signed-byte retries must remain one transaction trace with multiple
  submission attempts.
- A newly signed replacement must be a different trace even when it serves the
  same business action.
- One business action may contain zero, one, or many traces.
- Events must remain idempotent across SDK batch retries.
- IDs must be generated offline in SDK and server processes without a database
  round trip or central allocator.
- Rust, TypeScript, JSON Schema, PostgreSQL, CLI, and REST APIs need one canonical
  representation.
- Trace creation must be possible before a signature or signed-byte fingerprint
  exists.
- Signature/fingerprint availability depends on privacy mode and instrumentation
  completeness.
- Raw events must not be rewritten when trace aliases are discovered.
- Identity correlation must never merge projects, environments, clusters, or
  customer intentions using weak heuristics.
- Metric denominators and retry/replacement diagnoses must use the correct
  semantic level.

## Options considered

### Option A — Layered IDs plus environment-scoped correlation and aliases

Use separate random IDs for business actions, transaction traces, submission
attempts, and events. Use signature and a versioned signed-bytes fingerprint as
high-confidence correlation keys within one environment. When different local
trace IDs refer to the same signed transaction, retain one canonical trace and
map the others as aliases.

This represents the product semantics directly and supports pre-signing capture,
privacy modes, distributed generation, stable retries, and reconciliation. It
requires more entities, typed APIs, alias resolution, and careful merge tests.

### Option B — Solana signature as the trace primary key

Use the first transaction signature as the only trace identifier. This is
familiar to Solana engineers and makes explorer lookup easy, but the signature
is unavailable before signing and may be missing after a client timeout. Strict
privacy may prohibit central storage. It also makes application-side pre-signing
events awkward and couples Landfall's public identity to one chain-specific
value.

Signature remains an important correlation/search key, not the primary key.

### Option C — Signed-bytes digest as the trace primary key

Derive trace identity deterministically from signed transaction bytes. Identical
retries naturally converge, but the digest is unavailable before signing,
changes with fingerprint algorithm/privacy mode, and can become a correlation
handle with privacy implications. A digest is also not a convenient type-safe
identifier for every entity.

The digest remains a versioned high-confidence correlation key rather than the
public trace ID.

### Option D — Trust only the first SDK-generated trace ID

Require every process to propagate exactly one trace ID and never reconcile IDs
server-side. This is simple, but process boundaries, queues, worker restarts, or
manual instrumentation can generate two trace IDs for the same signed bytes.
Metrics would then double-count a single transaction and observers might poll it
twice.

Propagation remains the preferred path; aliases are the recovery mechanism.

### Option E — One ID for business action, transaction, retry, and event

Treat all activity for an order/payout as one record. This minimizes tables but
cannot distinguish repeated network calls from distinct signed replacements.
Route metrics, landing denominators, duplicate-success warnings, and evidence
links become ambiguous or incorrect.

### Option F — Database sequences or random UUIDv4 for all records

Database sequences prevent offline SDK generation and reveal local cardinality.
UUIDv4 supports distributed generation but has random B-tree insertion locality
and no useful creation-order hint. UUIDv7 provides a standardized time-ordered
layout with random uniqueness while preserving a normal UUID database type.

## Decision

Landfall shall use five distinct identity concepts:

1. **Business action** — one customer intention, such as a swap, payout, or
   position update. It may group multiple replacement transaction traces.
2. **Transaction trace** — one unique serialized signed transaction and its
   complete application, submission, observation, and execution lifecycle.
3. **Submission attempt** — one application-visible invocation that submits the
   same trace's signed bytes through one configured route.
4. **Event** — one immutable observed fact. Its ID is the authoritative
   ingestion idempotency key.
5. **Trace alias** — an alternate trace ID proven to refer to an existing
   canonical trace; it is a resolution mapping, not another transaction.

`BusinessActionId`, `TraceId`, `AttemptId`, and `EventId` shall be UUIDv7 values
generated with a cryptographically secure random source. PostgreSQL stores them
as native `UUID`. The canonical JSON/URL representation is the lowercase
RFC 9562 hex-and-dash form (`8-4-4-4-12`) with no braces or `urn:uuid:` prefix.
Rust and TypeScript shall use distinct newtypes/branded types so IDs cannot be
interchanged accidentally merely because all serialize as UUID strings.

UUID creation time is an indexing/debugging hint, not authoritative event order,
business time, or security proof. `occurred_at`, collector time, and semantic
ordering rules remain separate. An implementation must not derive authorization,
tenant ownership, or lifecycle conclusions from a UUID timestamp.

Trace equality is established within one environment by high-confidence
transaction correlation—normally the same versioned signed-bytes fingerprint or
the same permitted Solana signature. Different signed bytes always remain
different canonical traces. If they serve the same customer intention, they are
linked through one business action rather than merged.

## Detailed design and boundaries

### Entity hierarchy

```text
Environment
  |
  +-- BusinessActionId A                    "execute payout 42"
        |
        +-- TraceId T1                      signed bytes X
        |     +-- AttemptId P1              submit X via route R1
        |     +-- AttemptId P2              retry X via route R2
        |     +-- EventId E1..En            lifecycle evidence
        |
        +-- TraceId T2                      replacement signed bytes Y
              +-- AttemptId P3              submit Y via route R2
              +-- EventId Em..Ez            lifecycle evidence
```

The business action is optional because privacy policy or incomplete
instrumentation may prevent grouping. Trace identity is mandatory for
trace-scoped lifecycle events once the SDK creates a transaction context. Events
that describe only data quality or a business action follow their schema's
explicit scope rather than inventing a transaction trace.

### Identifier contract

| Identifier | Generated when | Stable across | Must change when | Primary purpose |
|---|---|---|---|---|
| `BusinessActionId` | Customer intention context is created | All replacement transactions for that intention | A new independent customer intention begins | Group technical outcomes under one intended operation |
| `TraceId` | Transaction context is created, before submission | Simulation/signing/submission/observation for one signed transaction; identical-byte retries | New signed bytes represent a replacement | Public Landfall handle for one transaction lifecycle |
| `AttemptId` | Immediately before one actual submission invocation | Start/result/receipt events for that invocation | Another route call or rebroadcast invocation begins | Measure transport/RPC behavior without inflating transaction count |
| `EventId` | One telemetry fact is created | Transport/batch retries of that exact fact | A new observation, correction, or fact is emitted | Immutable evidence identity and ingestion deduplication |
| Alias trace ID | A producer already emitted another `TraceId` | Lookups and historical events using that alternate ID | Never reused for another entity | Resolve alternate handles to a canonical trace |

Other internal entities may also use UUIDv7, but this ADR does not define their
domain semantics.

### Business action rules

The SDK may create a `BusinessActionContext` before any transaction exists and
propagate its `BusinessActionId` to each replacement trace. If multiple processes
must participate, they must propagate the same Landfall ID or a privacy-approved
external correlation value; independently generated IDs are not inferred to be
the same action.

A raw customer order, payout, user, or position identifier is not used as a
Landfall primary key and is not stored by default. If external correlation is
enabled, Landfall stores a versioned environment-scoped keyed digest or another
explicitly approved pseudonymous representation according to ADR-004. The
original external identifier is not required for product operation.

Landfall never groups business actions using amount, accounts, program IDs,
instruction similarity, timing proximity, or application label alone. Absence of
a business-action correlation produces “grouping unavailable,” not a guessed
relationship.

Business-action outcomes remain distinct from network outcomes. If two
replacement traces execute successfully, Landfall warns about multiple technical
successes but does not claim a duplicate payout/swap effect without
application-provided reconciliation evidence.

One trace belongs to at most one business action in P0. If the same trusted
signed-transaction correlation arrives with different business-action IDs, the
transaction is still counted as one trace, but Landfall records a business-action
correlation conflict and excludes ambiguous action-level conclusions until it is
resolved. It does not silently merge the business actions or choose the latest
value.

### Transaction trace rules

The SDK creates a `TraceId` before submission so construction, blockhash,
simulation, signing, and client-side failures can be recorded even if no
signature is returned. Once signed bytes are available, the SDK emits the
approved fingerprint and, where privacy permits, the Solana signature.

The logical invariant is one canonical trace per unique signed transaction in
one environment:

```text
same exact signed bytes + repeated submit calls
    => one canonical TraceId + multiple AttemptId values

different signed bytes + same BusinessActionId
    => multiple canonical TraceId values linked as replacements
```

A different recent blockhash, signature set, fee/compute-budget instruction, or
any other serialized signed-byte change creates a different trace. Landfall does
not infer that the new trace safely replaces the old one; both may still land.

Signature and fingerprint uniqueness is environment-scoped. No correlation may
cross project/environment/cluster boundaries merely because a value matches.
When both signature and fingerprint are present but imply different canonical
traces, Landfall records conflicting identity evidence and does not auto-merge
until the conflict is resolved by a deterministic rule or operator review.

### Submission attempt rules

An `AttemptId` represents one application-visible submission invocation. Its
start, result, duration, route, configuration, returned signature, transport
error, RPC error, or specialized receipt share that ID.

Every actual subsequent route invocation gets a new `AttemptId`, including a
retry of identical signed bytes and concurrent sends to multiple routes. Merely
scheduling a retry does not create an attempt until the submission call begins.
A provider's undocumented internal rebroadcast is not a new Landfall attempt;
it may appear as route evidence only when the provider exposes a receipt.

Attempt sequence numbers are display/order metadata, not identity. Concurrent
attempts can overlap and must not depend on a globally serialized counter.

### Event identity and transport retries

Every emitted fact gets a new `EventId` before it enters the SDK buffer or an
internal persistence path. That ID remains unchanged when the same event is
retried in a different HTTP request or batch. The collector's global dedup
registry uses `EventId` as the authoritative idempotency key.

Two independent observations of the same signature status are separate events
unless the observer deliberately coalesces them according to a documented event
contract. A correction is a new event and never reuses the ID of the fact it
clarifies or supersedes.

`batch_id` is a transport diagnostic identifier and may remain stable when an
entire batch is retried. It does not replace event-level deduplication. A server
`request_id` identifies one HTTP handling instance and has no domain meaning.

### Canonical trace and aliases

Producers should propagate one `TraceId`, but Landfall must tolerate this race:

```text
process A: TraceId TA + signed fingerprint F
process B: TraceId TB + signed fingerprint F
```

An existing trace already bound as the canonical owner of fingerprint/signature
F wins over a later claim. For concurrent first claims, the trace with the
earlier database creation time wins; an equal-time tie is resolved by lexical
UUID byte order. If two previously separate canonical groups are later proven to
be the same signed transaction, the older canonical binding wins, and all
aliases of the losing root are repointed directly in the same transaction. The
losing trace ID becomes an alias and is not automatically promoted again.

Alias invariants are:

- aliases are scoped to the same environment as the canonical trace;
- alias graphs contain no chains or cycles—every alias resolves directly to the
  current canonical root;
- a canonical trace never resolves to itself;
- raw events retain the originally supplied trace ID;
- projectors resolve all aliases before loading/reducing the canonical event set;
- API lookups by alias return the canonical identity and may expose the alias
  relationship where privacy permits;
- deletion/retention for a canonical trace includes events and derived data
  reachable through its aliases;
- alias creation records the reason and supporting correlation evidence so a
  merge is auditable.

Database uniqueness on `(environment_id, signature)` and on the approved
versioned fingerprint key serializes concurrent claims. A losing transaction
creates the alias and schedules canonical reprojection rather than rewriting raw
event foreign keys. Exact SQL and conflict handling belong to storage design and
must implement these invariants transactionally.

Low-confidence similarities never create aliases. Blockhash, accounts, amount,
program, instruction shape, route, timestamps, flow label, or business action
alone cannot prove that two trace IDs are the same signed transaction.

### Type and serialization boundaries

Conceptually, Rust uses transparent newtypes rather than plain `Uuid` parameters:

```rust
struct BusinessActionId(Uuid);
struct TraceId(Uuid);
struct AttemptId(Uuid);
struct EventId(Uuid);
```

TypeScript uses validated branded/opaque string types at public SDK boundaries.
Parsing validates UUID version and canonical form; generation uses an RFC 9562
compatible UUIDv7 library and a secure random source. Golden fixtures must prove
that Rust, TypeScript, JSON Schema, PostgreSQL, and OpenAPI serialize identical
lowercase values.

Nil UUIDs, wrong UUID versions, braces, URN prefixes, whitespace, and malformed
strings are rejected by event/API contracts. CLI convenience input may normalize
case before calling the contract, but generated wire data is canonical lowercase.
Clock rollback must not cause ID reuse; the selected libraries must document
their UUIDv7 randomness/monotonic behavior and pass clock-regression tests.

This ADR does not choose the exact Rust/TypeScript UUID libraries, fingerprint
algorithm/key policy, event JSON Schema, or public error-code vocabulary. It
fixes the identity semantics and wire representation those implementations must
preserve.

## Consequences

### Positive

- Landing and execution metrics count unique signed transactions rather than
  submission calls.
- Route/retry analysis retains every attempt without double-counting traces.
- Replacement transactions remain visible and can produce a multiple-success
  warning under one business action.
- Trace capture works before signature creation or RPC response.
- Stable event IDs make arbitrary batch retries idempotent.
- Alias resolution corrects distributed instrumentation races without rewriting
  immutable raw evidence.
- UUIDv7 permits offline generation, native PostgreSQL storage, time-local index
  insertion, and one cross-language representation.
- Typed IDs prevent common code defects such as passing an `EventId` where a
  `TraceId` is required.

### Negative

- The domain and database contain more entities, foreign keys, indexes, and API
  concepts than a signature-only design.
- Canonicalization and alias merges require transactional concurrency logic and
  replay of affected projections.
- UUIDv7 exposes an approximate creation timestamp and must not be mistaken for
  authoritative event time.
- Business-action grouping is incomplete when the customer cannot or will not
  provide correlation.
- Strict privacy may remove signature/fingerprint correlation and therefore make
  server-side alias recovery impossible.
- Users must understand that one business action, one trace, and one attempt are
  different metric denominators.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Alias poisoning merges unrelated traces | Lost evidence boundaries and incorrect metrics | Environment scope, high-confidence signature/fingerprint only, conflict checks, auditable reason/evidence, no heuristic merges |
| Same trace is counted twice during concurrent canonical claims | Inflated transaction and observer counts | Unique environment-scoped correlation indexes, transactional winner selection, direct alias, canonical reprojection |
| Replacement is treated as retry | Safety warning and transaction denominator are wrong | Signed-byte fingerprint comparison; any byte change creates a new trace; dedicated golden fixtures |
| Retry is treated as replacement | Landing denominator and costs are inflated | Reuse trace context for identical bytes; server alias recovery by signature/fingerprint; SDK adapter tests |
| Event ID changes during transport retry | Duplicate evidence and metrics | Assign ID before buffering; serialize stored envelope unchanged across retries; global dedup tests |
| Raw customer correlation ID leaks business data | Privacy and customer-identification exposure | Landfall UUID primary key, environment-keyed external digest, raw value disabled by default, redacted logs/search |
| UUID timestamp is trusted as event order | Incorrect lifecycle or latency | Separate occurred/received/monotonic fields; UUID only final deterministic tie-breaker where specified |
| UUID collision or generator defect is silently accepted | Two entities become one | CSPRNG-backed compliant libraries, uniqueness constraints, fail closed with observable collision error, cross-language test vectors |

## Security and privacy impact

Landfall IDs are opaque correlation handles, not authorization capabilities.
Possession of a UUID never grants access; every API lookup remains scoped and
authorized by project/environment credentials. Random UUIDv7 bits make casual
enumeration impractical, but access control must not depend on unguessability.

UUIDv7 reveals an approximate millisecond creation time. This is acceptable for
P0 internal/domain IDs, which already accompany timestamps, but IDs must not
embed customer IDs, account addresses, host identifiers, shard numbers, or
secrets. Logs and reports apply the same privacy policy to IDs as to other
correlation fields.

Raw external business identifiers are prohibited by default. A privacy-approved
correlation digest is scoped to an environment so two deployments or environments
cannot be joined by comparing stored values. ADR-004 defines the exact keyed
fingerprint policy.

Signature/fingerprint lookup and alias creation are environment-scoped and use
authenticated evidence. Strict mode may deliberately withhold these values; the
server then accepts reduced canonicalization capability and reports the data
quality limitation rather than weakening privacy.

## Reliability and failure behavior

IDs are created before asynchronous work begins and remain stable through SDK
buffering, HTTP retries, database retries, projector replay, and report creation.
No retry path may generate a replacement ID for the same logical entity merely
because its previous response was lost.

Concurrent inserts that claim the same signature/fingerprint resolve through a
database transaction and uniqueness constraint. One canonical trace wins; the
other ID becomes an alias. If alias creation or reprojection fails, the durable
raw events remain available and a retry can complete reconciliation.

Contradictory identity evidence does not silently choose a winner. The affected
trace remains queryable with an identity/data-quality warning, and automated
merge waits for a safe deterministic result. A merge operation is idempotent;
repeating it cannot create alias chains, cycles, or duplicate projections.

If a UUID generator reports entropy, clock, or collision failure, Landfall fails
that event/context creation explicitly rather than emitting a reused or malformed
ID. SDK failure policy remains fail-open for the customer's Solana transaction,
but it reports telemetry loss through its configured callback/counter.

## Performance and capacity impact

UUIDv7 uses PostgreSQL's 16-byte native UUID type and provides time-local B-tree
insertion compared with fully random UUIDv4. Time order does not replace explicit
indexes for event or business timestamps.

At the design point, one million events/day creates one million dedup keys/day,
while traces average 50,000/day and attempts approximately 60,000/day under the
1.2-attempt assumption. Required indexes include event ID deduplication,
environment-scoped signature/fingerprint uniqueness, business-action-to-trace,
trace-to-attempt, and alias-to-canonical resolution.

Aliases point directly to a canonical root so normal resolution is one indexed
lookup rather than recursive traversal. Alias conflict/merge work is expected to
be rare; an elevated alias rate is an instrumentation-quality signal, not a
normal scaling mechanism.

## Operational impact

Structured logs and system status should expose typed IDs needed for request,
job, and trace correlation while applying privacy/redaction policy. Metrics must
track alias creation, identity conflicts, UUID validation failures, duplicate
events, attempts per trace, and replacement traces per business action.

CLI/API trace lookup accepts a canonical or alias trace ID and reports the
canonical ID. Diagnostic tooling should explain why an alias was created without
printing prohibited fingerprint material. Operators need a safe inspection path
for identity conflicts before any manual merge capability is considered.

Schema migrations shall store IDs as PostgreSQL UUID rather than text and use
foreign keys where retention/partitioning permits. API schemas name each ID field
explicitly; a generic untyped `id` is avoided where it makes the entity unclear.

## Verification

- Cross-language RFC 9562 fixtures prove UUIDv7 version bits, canonical lowercase
  serialization, parsing rejection, and database round trips.
- SDK tests prove a trace ID exists before submission and remains stable across
  lifecycle capture and transport retry.
- Three submissions of identical signed bytes produce one canonical trace and
  three attempts, including concurrent and cross-route cases.
- Rebuilding/signing different bytes under one business action produces a second
  canonical trace and increments transaction—but not business-action—counts.
- Two producers emitting different trace IDs with the same trusted fingerprint
  converge transactionally to one canonical trace and one direct alias.
- Conflicting signature/fingerprint evidence does not auto-merge and creates a
  visible data-quality condition.
- Event batch retry preserves event IDs and does not duplicate raw evidence,
  projections, or metric contributions.
- Alias lookup, reprojection, report export, retention, and deletion include all
  events reachable through the canonical trace.
- Property tests prove alias graphs cannot contain cycles, self-links, chains,
  cross-environment edges, or two canonical roots for one trusted correlation.
- Privacy fixtures prove raw customer business IDs and prohibited correlation
  values do not enter events, logs, URLs, projections, or reports.

## Rollout and migration

This is the initial identifier model. Protocol and core implementation shall
first introduce distinct Rust newtypes and TypeScript branded types, followed by
golden UUIDv7 fixtures, context APIs, event fields, relational keys, correlation
indexes, and alias resolution.

During early schema development, all ID columns shall be created as UUID from the
start; no production text-to-UUID migration is planned. Existing documentation
fixtures using abbreviated placeholders remain illustrative and will be replaced
by valid canonical UUIDv7 values when executable fixtures are created.

Changing ID format or identity semantics after public release requires a
superseding ADR, dual-identifier API/migration period, alias/backfill strategy,
and explicit metric continuity analysis.

## Reconsideration triggers

Revisit this model if:

- a supported runtime cannot reliably generate RFC 9562 UUIDv7 values;
- strict privacy becomes the default and central signature/fingerprint
  correlation is normally unavailable;
- durable nonce, multisigner, bundle, or another supported submission primitive
  invalidates the one-signed-serialization-per-trace invariant;
- customer workflows require one transaction to satisfy multiple business
  actions or a business-action hierarchy;
- alias rates show that SDK context propagation is systematically failing;
- hosted multi-tenancy requires globally different external ID exposure or
  tenant-specific opaque public handles;
- measured B-tree/index behavior or identifier volume violates capacity targets.

Any replacement must preserve metric meaning for retries versus replacements and
provide a migration path for existing evidence references.

## References

- [RFC 9562: Universally Unique IDentifiers](https://www.rfc-editor.org/rfc/rfc9562.html)
- [Product requirements: terminology and semantic model](../product-requirements-document.md#9-terminology-and-semantic-model)
- [Product requirements: metric definitions](../product-requirements-document.md#104-metric-definitions)
- [Product requirements: instrumentation SDK](../product-requirements-document.md#132-instrumentation-sdk)
- [Product requirements: data model](../product-requirements-document.md#16-data-model)
- [System design: batch event ingestion](../system-design.md#64-batch-event-ingestion)
- [System design: trace identity and aliases](../system-design.md#86-trace-identity-and-aliases)
- [Technical implementation plan: event protocol](../technical-implementation-plan.md#9-phase-2--versioned-event-protocol)
- [Technical implementation plan: domain core](../technical-implementation-plan.md#10-phase-3--deterministic-domain-core)
