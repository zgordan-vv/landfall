# ADR-006: Solana Kit first adapter and compatibility roadmap

- **Status:** Accepted
- **Date:** 2026-08-30
- **Decision owners:** Landfall maintainers
- **Related requirements:** FR-SDK-001 through FR-SDK-006, FR-PRIV-001 through FR-PRIV-003
- **Related ADRs:** [ADR-003](003-business-action-trace-attempt-event-and-alias-identifiers.md), [ADR-004](004-privacy-modes-and-signed-byte-fingerprints.md), [ADR-005](005-json-schema-event-contract-and-code-generation.md)
- **Supersedes:** None
- **Superseded by:** None

## Context

Landfall must observe a real Solana transaction lifecycle from a customer's
Node.js application. The useful evidence is distributed across several client
operations: acquiring a recent blockhash, simulating, signing, serializing,
submitting one or more attempts, waiting for confirmation, and recording the
application outcome. A generic HTTP interceptor can see only part of that
lifecycle and cannot reliably distinguish a retry of identical signed bytes
from a replacement transaction.

The server protocol is intentionally client-library-neutral. Nevertheless, a
portfolio implementation needs one high-quality integration that demonstrates
low-friction instrumentation against a real, current Solana client. The choice
of that first adapter affects public TypeScript APIs, dependency compatibility,
privacy boundaries, failure isolation, test fixtures, documentation, and the
cost of adding legacy clients later.

The Solana JavaScript ecosystem is evolving quickly. Current official guidance
uses the plugin-based `@solana/kit` client for new applications. Earlier project
documents used the broad label `@solana/kit` v7+ and named
`@solana/web3-compat` as a possible bridge. Current Solana migration guidance
instead treats those earlier framework-kit compatibility packages as
superseded, points new applications to Kit, and describes `@solana/web3.js` v3
as the class-based migration bridge while it remains a release candidate.

Treating all Kit releases as one compatible `v7+` surface would therefore be
unsafe. Landfall also cannot promise compatibility merely because two packages
share Solana types: instrumentation depends on concrete lifecycle boundaries
and behavior, not only TypeScript assignability.

## Decision drivers

- Produce a credible P0 integration for the current Solana JavaScript stack.
- Keep the neutral event protocol and SDK core independent of Kit types and
  release cadence.
- Preserve the customer's transaction semantics, error identity, timeout,
  commitment, retry policy, and route choice.
- Keep telemetry network I/O off the transaction critical path and satisfy the
  SDK synchronous p95 overhead target of less than 5 ms.
- Never accept signer secret material or persist raw signed transaction bytes.
- Capture only evidence the adapter actually observes; missing stages must not
  be inferred from a high-level client result.
- Make supported versions explicit, reproducible, testable, and actionable when
  a customer uses an unsupported combination.
- Avoid multiplying P0 test and support work across modern, migration, legacy,
  browser, and wallet APIs before customer evidence justifies it.
- Leave a practical path to add adapters without changing the ingestion schema
  or domain model.

## Options considered

### Option A — Kit-first explicit adapter over the neutral SDK

The neutral SDK owns trace context, event construction, privacy, buffering, and
transport. A Kit-specific subpath supplies typed wrappers for observable Kit
operations and translates their results into neutral events. It never patches a
global, prototype, signer, or RPC transport.

This targets the recommended ecosystem while confining change-prone Kit APIs to
one module. It requires applications to place explicit wrappers at lifecycle
boundaries, but those call sites make the captured evidence and missing evidence
honest and reviewable.

### Option B — Legacy `@solana/web3.js` v1 first

This could reach a large installed base and its class-based `Connection` API is
familiar. It would make the portfolio lead with a legacy API, add pressure to
monkey-patch or proxy a large class, and delay proving integration with the
recommended stack. It also would not remove the need for a separate Kit
adapter.

### Option C — Neutral manual event API only

Applications could explicitly report every lifecycle stage without any client
dependency. This remains an essential fallback and is implemented before the
adapter, but by itself it does not demonstrate low-friction integration or prove
that Landfall can safely obtain canonical signed bytes from a real client flow.

### Option D — Ship Kit, web3.js v3, and web3.js v1 adapters together

This maximizes apparent compatibility at launch. It also triples the volatile
integration surface, fixtures, examples, release matrix, and user support
before there is evidence that all three are needed. The web3.js v3 bridge is
currently pre-stable, so promising it as a production P0 lane would be
especially expensive.

### Option E — Transparent monkey-patching or transport interception

Landfall could replace client methods, patch prototypes, or intercept JSON-RPC
transport globally. Integration might look smaller, but observation would be
incomplete and sensitive to undocumented internals. It could change `this`
binding, error identity, cancellation, serialization, or retry behavior and
would create a high-risk supply-chain-like position inside customer payment
code.

## Decision

Landfall will implement the neutral manual TypeScript SDK first and then ship
one P0 Solana-specific adapter for the server-side `@solana/kit` client/plugin
generation on Node.js 24 LTS.

The adapter will be exposed through a Kit-specific package subpath, separate
from the neutral root entry point. It will use explicit composition around
documented application operations. It will not monkey-patch Kit, replace RPC
transport globally, wrap signer implementations, or own the customer's retry,
confirmation, and routing policies.

The initial production compatibility lane targets Kit 8.x. Exact Kit and plugin
minor/patch versions will be pinned in the lockfile, examples, and CI and listed
in the Landfall support matrix before the adapter is released. A broad `v7+`
claim is not a support contract. The public peer range must include only release
lines exercised by the compatibility suite.

The neutral manual API remains the fallback for every unsupported client. A
web3.js v3 compatibility spike may follow the Kit adapter, but production
support waits for a stable-enough upstream release and/or pilot demand. A
separate legacy web3.js v1 adapter is implemented only when customer evidence
justifies its permanent test and maintenance cost. The superseded
`@solana/web3-compat` package is not a roadmap target.

## Detailed design and boundaries

### Dependency direction

The allowed dependency direction is:

```text
customer application
        |
        v
Kit adapter subpath  --->  @solana/kit and selected Kit plugins
        |
        v
neutral Landfall SDK ---> generated neutral event DTOs
        |
        v
async SDK buffer and ingestion transport
```

The neutral SDK root must not import, re-export, or mention Kit runtime types.
The collector, projector, domain core, database, and dashboard must never import
Kit. All Kit objects are normalized at the adapter boundary into the canonical
event contract from ADR-005.

For P0, the intended source layout is:

```text
packages/sdk-ts/src/
├── core/                    # Context, event emission, buffering, transport
├── adapters/
│   └── solana-kit/          # The only module allowed to import Kit
│       ├── compatibility.ts # Supported capability/version checks
│       ├── trace.ts         # Kit-specific trace facade
│       ├── stages.ts        # Explicit lifecycle wrappers
│       ├── normalize.ts     # Kit result/error to neutral fields
│       └── index.ts         # Kit adapter public surface
└── index.ts                 # Neutral public surface; no Kit import
```

The published package exposes the adapter only from a dedicated subpath such as
`@landfall/sdk/solana-kit`; the exact npm scope/name is a packaging detail. The
subpath boundary is binding. If peer-versioning or release cadence becomes
independent, the same module may move to a separate adapter package without
changing the neutral event contract.

### Explicit observation instead of hidden interception

The adapter accepts application-owned operations as callbacks or equivalent
explicit dependencies. An illustrative, non-final API shape is:

```ts
const trace = kitAdapter.startTrace({ flow: "checkout" });

const lifetime = await trace.blockhash(() =>
  rpc.getLatestBlockhash().send()
);

const simulation = await trace.simulation(() =>
  rpc.simulateTransaction(transaction).send()
);

const signedTransaction = await trace.signing(() =>
  signTransactionMessageWithSigners(transactionMessage)
);

const signature = await trace.submission({ routeId: "primary" }, () =>
  sendTransaction(signedTransaction)
);

await trace.confirmationWait(() => waitForConfirmation(signature));
```

The names and overloads can change during API design. The following semantics
cannot:

1. The application callback is invoked exactly once by a stage wrapper.
2. The wrapper adds no retry, timeout, commitment, route selection, preflight,
   or confirmation policy.
3. A successful callback returns the original value without cloning or
   replacing it.
4. A synchronous throw or asynchronous rejection rethrows the original error;
   normalized telemetry never replaces the customer error.
5. Telemetry construction, buffering, redaction, or transport failure does not
   prevent the callback from running and does not change its result.
6. Duration is measured with a monotonic clock; wall-clock timestamps remain
   separate event fields.
7. Only structured allowlisted values cross into the neutral SDK.

When an application calls a high-level Kit helper such as
`client.sendTransaction`, internals such as planning, blockhash acquisition,
simulation, signing, and submission may be hidden inside that one call. A
wrapper around it may record only the stages it directly observes. It must not
synthesize inner lifecycle events. Applications needing a complete trace use
the granular Kit composition points or add manual neutral events around their
own stages.

### Lifecycle evidence captured

The adapter maps observable evidence to the canonical schema as follows:

| Application boundary | Evidence captured | Result and purpose |
|---|---|---|
| Trace creation | Trace/business-action context, flow, app/SDK/adapter versions | Establishes correlation before any submission exists |
| Recent blockhash | Blockhash, `lastValidBlockHeight`, route ID, timing | Defines the transaction lifetime evidence used for expiration diagnosis |
| Simulation | Started/finished timing, success/failure, compute units and structured configuration when returned | Distinguishes deterministic simulation failure from later landing problems |
| Signing | Started/finished timing and status only | Measures signing delay without observing signer secrets |
| Post-sign serialization | Exact serialized signed bytes, used only in memory for ADR-004 HMAC | Distinguishes rebroadcasts from replacement transactions |
| Submission attempt | Attempt ID, route ID, safe RPC settings, timing, returned signature or normalized error | Records every actual send and its immediate client outcome |
| Confirmation wait | Commitment, application wait timing and outcome | Separates application timeout from on-chain outcome |
| Business outcome | Explicit application-supplied result | Connects transaction lifecycle to user-visible success or failure |

Unavailable values remain unavailable. For example, skipped preflight does not
produce a successful simulation event, and a high-level send result does not
prove that the application waited for confirmation.

### Signing and signed-byte boundary

The adapter never accepts a private key, seed phrase, raw keypair, signer secret,
or callback that exposes secret key material. It observes signing only by timing
the application-supplied operation and receiving the already signed transaction
result that the application itself needs.

After all required signatures are present, the adapter obtains the exact
serialized signed transaction bytes through the documented Kit serializer. It
immediately computes the versioned environment-keyed HMAC from ADR-004, records
only the fingerprint envelope, and releases its reference to the bytes. It does
not put the bytes in an event, buffer, log, exception, diagnostic callback, test
snapshot, or custom metadata.

JavaScript cannot guarantee that every engine or library copy of a byte array is
physically erased. The adapter overwrites an owned temporary buffer when safe,
avoids extra copies, and documents that this is best-effort memory hygiene, not
cryptographic erasure. It must not mutate a buffer still owned or used by the
customer's submission call.

If the supported serializer cannot produce canonical signed bytes for a
transaction type, the adapter emits no fingerprint, reports an actionable local
completeness diagnostic, and leaves the customer operation unchanged. It never
falls back to hashing JSON, base64 text, an unsigned message, or a transaction
object.

### Route and error normalization

Applications configure stable, low-cardinality `route_id` labels separately
from RPC endpoint URLs. The adapter may use a URL locally to choose a route but
must remove credentials, query strings, authorization headers, and provider
tokens before event construction. Raw endpoint URLs are not an automatic event
field.

Kit and provider errors are converted through an allowlist into neutral error
category, stable code when known, retryability classification when justified,
and bounded safe message metadata. Landfall does not serialize an arbitrary
error object, cause chain, request body, headers, stack, or complete RPC
response. The original error is returned to the application unchanged while a
separate safe representation is sent to Landfall.

Unknown error shapes are classified as unknown with a local diagnostic. They do
not trigger unrestricted object serialization.

### Compatibility contract

Compatibility is declared as a tested lane, not inferred from a loose semver
claim. Each support-matrix row contains at least:

- Node.js release line;
- Landfall SDK version;
- Kit adapter version;
- `@solana/kit` exact tested version or narrow tested minor range;
- versions of Kit plugins used by the integration surface;
- supported transaction/message versions;
- supported operations and known capture gaps;
- support status and planned end date when deprecated.

The adapter's development dependencies and example lockfile use exact versions.
Its peer range includes only compatible release lines covered by CI. A minor
range may be advertised only after the lowest and highest accepted versions
pass the same compile, fixture, and integration suite.

Adapter initialization validates the capabilities it needs and produces a
stable, actionable incompatibility error before an operation is wrapped. The
SDK `doctor` command reports the resolved dependency lane, adapter version,
schema version, Node runtime, privacy mode, and missing capabilities. TypeScript
structural compatibility alone is not evidence of runtime support.

An unsupported transaction/message version is handled separately from an
unsupported Kit package version. The customer's operation remains under the
customer's control, but Landfall does not claim complete capture and does not
invent a fingerprint or parser result. The exact P0 transaction-version list is
owned by the support matrix and its fixtures, not by this ADR.

### Compatibility roadmap

The roadmap order is binding unless this ADR is superseded:

1. Ship the neutral manual API so every client has a safe fallback.
2. Ship the Node.js 24 + Kit 8.x adapter against an exact tested dependency
   lane, a controlled example, and local-validator integration tests.
3. Run a time-boxed `@solana/web3.js` v3 spike after the Kit lane is stable.
   Production support requires a sufficiently stable upstream surface or
   confirmed pilot demand and its own compatibility row and adapter boundary.
4. Add a legacy web3.js v1 adapter only after interviews, repositories, or a
   paying pilot show that it materially reduces adoption friction. It remains a
   separate module and never leaks legacy classes into the neutral SDK.
5. Treat browser/wallet instrumentation as a later product surface with its own
   security review, packaging constraints, and support matrix.

`@solana/compat` may be evaluated internally for safe data conversion, but using
it does not by itself make a client version supported. The earlier
`@solana/web3-compat` roadmap reference will be removed when the PRD, system
design, and implementation plan are reconciled at the end of Phase 0.

### What this ADR does not decide

This record does not finalize:

- public method names or npm scope;
- the exact first Kit 8.x minor/patch and plugin tuple;
- the initial supported Solana transaction/message versions;
- browser and wallet support;
- specialized submission-provider adapters such as Jito;
- observer-side RPC behavior;
- event field definitions already owned by ADR-005;
- fingerprint format and privacy modes already owned by ADR-004.

Those decisions must respect the boundaries established here and be recorded in
the support matrix, API documentation, schemas, threat model, or later ADRs as
appropriate.

## Consequences

### Positive

- The portfolio demonstrates the current recommended Solana JavaScript stack.
- Kit churn is isolated from the neutral event protocol and backend.
- Explicit wrappers provide truthful stage boundaries and preserve customer
  control over transaction behavior.
- Manual events keep Landfall usable for custom, legacy, and other-language
  clients before dedicated adapters exist.
- A narrow tested support lane produces reproducible bugs and actionable support
  instead of ambiguous `v7+` compatibility claims.
- Separate future adapters can reuse identity, privacy, buffering, transport,
  ingestion, and projection code.

### Negative

- Explicit instrumentation requires more application changes than transparent
  monkey-patching.
- A customer using only a high-level Kit send helper may receive a partial trace.
- Narrow version lanes require frequent compatibility testing as Kit evolves.
- Delaying legacy adapters may exclude otherwise interested early users.
- The SDK must maintain both neutral manual APIs and typed adapter APIs.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Kit minor changes a result or plugin shape | Compile failure or silently missing telemetry | Exact locks, narrow peers, capability checks, matrix CI, golden fixtures |
| Wrapper alters customer semantics | Failed or behaviorally different transaction | Exactly-once callback contract, original result/error tests, no added policy, failure-injection suite |
| High-level Kit API hides stages | Misleading diagnosis | Record only observable facts, surface completeness gaps, document granular integration |
| Signed bytes reach logs or telemetry | Sensitive payload disclosure | ADR-004 boundary, outbound-payload assertions, secret fixtures, log/snapshot scans |
| Adapter receives signer secrets for convenience | Expanded key-custody threat boundary | Callback boundary accepts no signer/keypair secret; API and type review |
| Endpoint credentials appear in errors | Credential disclosure | Local route mapping, allowlisted error normalization, adversarial URL/header fixtures |
| Version mismatch discovered during a transaction | Runtime surprise | Adapter initialization and `doctor` validate before wrapping production calls |
| Supporting pre-stable web3.js v3 consumes roadmap | P0 delay and brittle support | Time-boxed spike only after Kit; production gate requires evidence and dedicated lane |
| JavaScript byte cleanup is overclaimed | False privacy assurance | Avoid copies, best-effort overwrite owned buffers, document memory limitation |

## Security and privacy impact

The adapter runs inside the customer application and therefore sees public
transaction construction data and, briefly, the already signed bytes needed by
the application. It does not become a signer and does not receive private key
material. This boundary must be represented in the threat model.

Privacy classification and mode enforcement occur before neutral event creation
where possible and again in the neutral SDK. The adapter uses an allowlist; it
does not serialize Kit transaction objects, signer objects, RPC clients, arbitrary
errors, or application objects. Strict-mode restrictions from ADR-004 still
apply even if Kit makes additional values available.

The environment fingerprint key is a Landfall secret distinct from wallet and
RPC credentials. It is used only for the domain-separated HMAC specified in
ADR-004. Raw signed bytes and secret-bearing URLs are prohibited from logs,
diagnostic callbacks, crash metadata, events, and SDK buffers.

Adding a browser/wallet adapter would introduce different origin, extension,
storage, CSP, and user-consent risks and therefore requires a separate threat
review rather than inheriting P0 server-side approval.

## Reliability and failure behavior

The application operation is authoritative; telemetry is best effort. Every
wrapper is structured so that telemetry preparation failure is contained, the
application callback is still called once, and its value or original error is
preserved. The adapter does not retry the application operation.

Event emission uses the neutral SDK's bounded asynchronous buffer. Collector
outage, HTTP timeout, authentication failure, schema rejection, queue overflow,
or process-shutdown flush failure is reported through SDK diagnostics and
counters but is not thrown from the wrapped transaction call by default.

An adapter/API incompatibility is a setup error, not a transient telemetry
failure. It is reported during adapter construction or `doctor`, before the
application relies on instrumentation. Customers can continue with the neutral
manual API or no adapter; Landfall must not silently label an untested client as
supported.

If a stage cannot be observed, normalization fails, or fingerprinting is
unavailable, Landfall records only safe evidence it can prove and marks the
trace or local diagnostic as incomplete. It does not fabricate a success,
simulation, signature, attempt, or confirmation event.

## Performance and capacity impact

The adapter adds timers, bounded normalization, identifier generation, event
enqueueing, and one HMAC over the signed transaction bytes. It performs no
Landfall network request synchronously with the customer's RPC call. Event
batching and ingestion occur asynchronously through the neutral SDK.

The P0 acceptance target remains less than 5 ms p95 synchronous SDK overhead,
excluding the customer's original Kit/RPC operation, measured on the documented
reference Node.js 24 environment. Benchmarks report stage-wrapper overhead and
post-sign serialization/HMAC separately so a regression cannot be hidden by
network latency.

Fields and labels are bounded by ADR-005. Version, adapter, route, flow, and
policy labels use controlled values; the adapter does not turn addresses,
signatures, error messages, or endpoint URLs into metric labels.

Reconsider the design if the wrapper/HMAC budget is exceeded under representative
transactions, if a necessary Kit API requires large repeated byte copies, or if
real applications cannot expose enough granular boundaries for useful traces.

## Operational impact

The repository must pin the chosen Kit and plugin versions in its lockfile and
run the compatibility matrix in CI. Release notes and support documentation
state additions and removals of support lanes. Dependency update automation may
open changes but cannot widen the peer range or support matrix without the full
suite passing and maintainer review.

Every adapter-emitted trace carries Landfall SDK and adapter versions. The
`doctor` output provides an actionable command or documentation link for an
unsupported tuple and never prints endpoint credentials, signed bytes, or
fingerprint keys.

Local development uses a controlled example and local validator. Devnet tests
are explicit opt-in because they depend on external state; mainnet is never
required for CI. Compatibility failures are distinguishable from collector,
schema, RPC, and application failures.

## Verification

ADR-006 is implemented only when all of the following are true:

- The neutral SDK root builds and its dependency graph contains no Kit runtime
  import.
- The Kit subpath builds against every advertised support-matrix lane and fails
  with an actionable diagnostic against explicit unsupported fixtures.
- Compile tests cover the lowest and highest version allowed by each advertised
  peer range.
- Golden lifecycle tests assert exact neutral events for blockhash, simulation,
  signing, same-bytes resubmission, replacement, submission failure,
  confirmation timeout/later success, expiration, and business outcome.
- Wrapper contract tests prove exactly-once callback invocation, original value
  identity, original error identity, synchronous throw behavior, promise
  rejection behavior, and unchanged cancellation/timeout inputs.
- Failure injection proves collector outage, buffer overflow, redaction failure,
  fingerprint failure, and diagnostic-callback failure cannot prevent the
  customer operation.
- Privacy fixtures prove that private keys, raw signed bytes, credential-bearing
  URLs, headers, full transaction objects, stacks, and arbitrary RPC responses
  never appear in outbound payloads, logs, snapshots, or diagnostics.
- Fingerprints match ADR-004 golden vectors for exact signed bytes and differ for
  replacement bytes.
- A local-validator integration covers a documented successful transaction and
  deterministic failure scenarios without mainnet funds.
- Unsupported transaction/message versions pass through customer-owned
  operations without Landfall claiming complete capture.
- Node.js 24 benchmarks show p95 synchronous overhead below 5 ms and report
  serialization/HMAC separately.
- A fresh user can instrument the controlled example and produce a complete
  trace in under 30 minutes using only the public documentation.

## Rollout and migration

Rollout order is collector/schema compatibility first, neutral SDK second, Kit
adapter third, and example/support documentation last. This prevents a producer
from emitting events the deployed collector does not understand.

The first adapter release starts with one exact tested Kit 8.x lane. A wider
minor range is added only after matrix evidence. Upgrading Kit uses a pull
request that updates the exact development lock, support matrix, compile
fixtures, golden events, local-validator results, benchmarks, and known gaps.

Removing a previously supported lane requires documented deprecation and a
replacement path: upgrade Kit, remain on an explicitly maintained Landfall
adapter version, or use the neutral manual API. Raw historical events require no
migration because their schema is neutral and retains the producer/adapter
version as evidence.

If the Kit adapter is unsafe before general release, it can be removed while the
neutral SDK remains usable. After release, rollback means restoring the last
tested adapter/Kit tuple; it never requires a backend data rewrite.

## Reconsideration triggers

- Pilot repositories predominantly use a client other than Kit and manual
  instrumentation materially blocks adoption.
- `@solana/web3.js` v3 becomes stable and is widely adopted, or a paying pilot
  requires its class-based migration surface.
- More than 25% of qualified prospects require legacy web3.js v1 support.
- Official Kit APIs remove the explicit boundaries needed for useful lifecycle
  capture or supply a stable first-party instrumentation hook that is safer than
  wrappers.
- A supported Kit release cannot meet the less-than-5-ms p95 synchronous
  overhead target.
- Browser/wallet demand becomes P0 and requires a different packaging or threat
  model.
- Maintaining multiple Kit minor lanes consumes more effort than a separately
  versioned adapter package would.
- Solana changes transaction formats in a way that invalidates canonical signed
  byte serialization or the current support-matrix assumptions.

## References

- [P0 support matrix](../support-matrix.md)
- [Product requirements: instrumentation SDK](../product-requirements-document.md#132-instrumentation-sdk)
- [System design: application instrumentation](../system-design.md#32-application-instrumentation)
- [System design: TypeScript SDK](../system-design.md#91-typescript-sdk)
- [Technical implementation plan: Solana Kit adapter](../technical-implementation-plan.md#16-phase-9--solana-kit-adapter-and-example-application)
- [Official Solana Kit client guide](https://solana.com/docs/frontend/client)
- [Official Solana migration guidance](https://solana.com/docs/frontend/web3-compat)
- [Anza Kit repository](https://github.com/anza-xyz/kit)
- [ADR-003: transaction identity model](003-business-action-trace-attempt-event-and-alias-identifiers.md)
- [ADR-004: privacy and fingerprint policy](004-privacy-modes-and-signed-byte-fingerprints.md)
- [ADR-005: neutral event schema authority](005-json-schema-event-contract-and-code-generation.md)
