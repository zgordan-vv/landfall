# Landfall Product Requirements Document

Status: Draft; implementation is conditional on validation gates  
Version: 0.1  
Date: 2026-08-29  
Product owner: Founder  
Working product name: **Landfall**  
Related documents: [Idea Validation Strategy](./idea-validation-strategy.md), [System Design](./system-design.md), [Technical Implementation Plan](./technical-implementation-plan.md)

## 1. Executive summary

Landfall is a vendor-neutral observability and diagnostic system for the end-to-end lifecycle of Solana transactions.

It connects events that exist only inside an application—construction, simulation, signing, submission, retries, route selection, and business intent—with RPC responses and on-chain status observations. It turns those events into an evidence-linked timeline, reliability metrics, diagnoses with explicit confidence, and deterministic recommendations.

The first release is an open-source, self-hosted product designed for one technical team and one or more environments. It consists of:

- a small TypeScript instrumentation SDK for common Solana client flows;
- a Rust collector and lifecycle-analysis engine;
- PostgreSQL-backed trace storage;
- a local web dashboard;
- a Rust CLI for health checks and portable reports;
- adapters for standard Solana JSON-RPC, with specialized submission paths added only when required by a validated pilot.

Landfall does **not** hold private keys, sign transactions, promise that transactions will land, replace RPC infrastructure, or claim certainty where the available evidence is incomplete.

The commercial wedge is a fixed-scope Solana Transaction Reliability Audit. The software should first support real audits and paid pilots. A multi-tenant hosted SaaS should be built only if the validation strategy demonstrates recurring demand.

## 2. Document purpose and authority

This PRD defines:

- the product problem and intended customer;
- product goals and non-goals;
- terminology and metric semantics;
- minimum viable release scope;
- functional and non-functional requirements;
- lifecycle and diagnostic behavior;
- privacy and security requirements;
- architecture boundaries;
- rollout, testing, and acceptance criteria;
- known risks and unresolved decisions.

This PRD is deliberately more detailed than the first implementation plan. Requirements labeled P0 define the intended open-source MVP, but implementation of the full P0 scope should not begin before Gate A in the validation strategy passes. Requirements may be narrowed by customer evidence. Validation evidence overrides unsupported assumptions in this document.

## 3. Background

### 3.1 The fragmented transaction lifecycle

A production Solana transaction crosses several distinct systems:

1. Application code creates a business action and transaction message.
2. The application obtains a recent blockhash and configuration.
3. The transaction may be simulated.
4. A wallet, keypair, or remote signer signs it.
5. The application submits the signed bytes through an RPC endpoint or specialized sender.
6. The submission path may forward or rebroadcast the transaction.
7. A validator may receive, schedule, and execute it.
8. A status may become visible at processed, confirmed, and finalized commitment.
9. The application reconciles the network outcome with its business action.

Different tools observe different portions of this sequence. A block explorer cannot display a transaction that never entered a block. An application log often does not know whether an accepted RPC request was forwarded effectively. A fee estimator recommends a value but does not necessarily measure the result for the customer's own transaction types. A provider dashboard may not correlate events from another provider or from the customer's signing workflow.

### 3.2 User problem

When a transaction is delayed, absent, expired, or unsuccessful, an engineer must answer questions such as:

- Was the transaction correctly constructed and signed?
- Did simulation reveal an error?
- Was the blockhash fresh at signing and submission?
- Did the selected RPC or sender accept the request?
- Was the same signed transaction retried, or was a new transaction created?
- Did the transaction appear on-chain and fail during execution?
- Did the validity window pass without an observed inclusion?
- Was the compute limit inappropriate?
- Was the fee uncompetitive for the relevant local contention?
- Did an RPC or websocket view lag behind another source?
- Did the business action succeed even though the client timed out?
- Is retrying safe?

Today, teams may answer these questions by searching application logs, calling RPC methods manually, inspecting explorers, consulting provider dashboards, and comparing timestamps by hand. The result can remain uncertain because necessary pre-submission evidence was never recorded.

### 3.3 Product opportunity

Landfall creates value by making diagnostic evidence available before an incident and presenting it with honest certainty levels. It should reduce time spent reconstructing transaction history, expose reliability regressions, and help teams measure whether configuration or route changes improve results.

### 3.4 Competitive boundary

Infrastructure providers already offer transaction submission, priority-fee estimates, staked routing, Jito access, retries, and enhanced transaction APIs. Landfall must complement these tools rather than recreate them.

Landfall's differentiating boundary is:

- customer-controlled instrumentation before submission;
- correlation across application, route, and on-chain observations;
- provider-neutral definitions and comparisons;
- evidence-linked diagnoses with explicit uncertainty;
- privacy-preserving self-hosted operation;
- before-and-after measurement for the customer's own workload.

## 4. Product vision

### 4.1 Vision statement

Every production Solana team should be able to explain what happened to an important transaction, how confidently it knows, and whether a proposed reliability change actually worked.

### 4.2 Product promise

For every instrumented transaction, Landfall will provide:

- one coherent lifecycle timeline;
- a clear terminal or current state;
- the evidence collected and data that is missing;
- a confirmed, probable, or unknown diagnostic classification;
- safe, deterministic recommendations when supported;
- aggregate metrics that distinguish submission, landing, execution, and business success.

### 4.3 Product principles

1. **Evidence before explanation.** Every diagnosis links to observable events.
2. **Unknown is a legitimate result.** The product must not invent causality.
3. **Never endanger transaction delivery.** Telemetry is non-blocking and fails open.
4. **No custody and no signing authority.** Private keys never enter Landfall.
5. **Vendor neutrality.** Metrics use the same definitions across submission paths.
6. **Local-first privacy.** The useful initial product can run inside a customer's environment.
7. **Measure outcomes, not feature activity.** Landing, latency, cost, and investigation time matter more than dashboard views.
8. **Recommendations are reviewable.** The MVP recommends changes but does not silently mutate production transactions.
9. **Business actions and network transactions are different.** Idempotency and correlation must be explicit.
10. **Build from paid evidence.** Repeated pilot needs determine the hosted roadmap.

## 5. Goals and non-goals

### 5.1 P0 goals

G1. Instrument a standard server-side Solana transaction flow in less than 30 minutes.

G2. Reconstruct a lifecycle timeline from creation or pre-submission through a terminal observed state.

G3. Correctly separate transaction construction/submission problems, non-inclusion/expiration, and on-chain execution failures.

G4. Show confirmed, probable, and unknown diagnoses with supporting evidence and limitations.

G5. Calculate stable, documented reliability and latency metrics without counting retries as independent business successes.

G6. Generate deterministic recommendations for common configuration and observability deficiencies.

G7. Compare results by transaction flow, application version, environment, and submission route.

G8. Operate self-hosted without collecting private keys or raw signed transactions by default.

G9. Export a portable, sanitized diagnostic report suitable for a reliability audit or customer review.

G10. Demonstrate the complete product with reproducible local-validator, devnet, and controlled mainnet fixtures.

### 5.2 P1 goals after a validated pilot

- specialized Jito transaction and bundle lifecycle support;
- route health comparisons across multiple RPC/sender endpoints;
- configurable regression alerts;
- OpenTelemetry export;
- longer retention and scheduled reports;
- optional client-side/browser instrumentation;
- optional Rust application SDK;
- production-grade self-hosted deployment documentation.

### 5.3 Future goals conditional on recurring demand

- hosted multi-tenant SaaS;
- organization, team, and role management;
- enterprise SSO and audit logging;
- anonymized fleet-wide baselines with explicit customer consent;
- automated experiment analysis;
- controlled policy application with approval workflows;
- white-label or provider partnership integrations.

### 5.4 Non-goals

NG1. Acting as a wallet, signer, custodian, facilitator, validator, or RPC provider.

NG2. Storing private keys, seed phrases, remote-signer credentials, or wallet recovery material.

NG3. Guaranteeing inclusion, execution, confirmation time, trading profit, or financial outcome.

NG4. Providing a general-purpose block explorer or historical chain indexer.

NG5. Replacing security audits of Solana programs.

NG6. Determining a definitive cause for every absent transaction.

NG7. Automatically resubmitting or rebuilding transactions in P0.

NG8. Automatically changing priority fees, compute limits, routes, or business idempotency policy in P0.

NG9. Offering trading signals, token recommendations, MEV strategy, or market data.

NG10. Supporting every language, wallet, RPC extension, durable nonce, and bundle type in the first release.

NG11. Using an LLM as the source of truth for diagnostic classification. An LLM may later summarize already structured evidence, but deterministic logic remains authoritative.

## 6. Target users and personas

### 6.1 Primary persona: infrastructure engineer

**Context:** Maintains a backend that submits thousands of Solana transactions through one or more endpoints.

**Goals:**

- know why reliability changed;
- investigate a specific signature or business action;
- compare routes and application releases;
- tune configuration without overpaying;
- avoid maintaining another bespoke tracing system.

**Frustrations:**

- explorers lack pre-inclusion data;
- application logs use inconsistent identifiers;
- provider metrics are difficult to compare;
- missing timestamps make retrospective analysis impossible;
- retry behavior obscures the actual number of intended actions.

**Success statement:** “I can locate an incident, see the complete evidence in one timeline, and know which next action is justified.”

### 6.2 Primary economic buyer: CTO or technical founder

**Context:** Leads a small production team where transaction reliability affects customers or revenue.

**Goals:**

- improve reliability without hiring a protocol specialist;
- reduce incident and support cost;
- make provider and architecture decisions using evidence;
- obtain a credible before-and-after report.

**Frustrations:**

- engineers spend days on inconclusive investigations;
- vendors make incompatible performance claims;
- failures are discussed anecdotally rather than measured;
- technical incidents cannot be translated into business impact.

**Success statement:** “The team can demonstrate what improved, by how much, and at what cost.”

### 6.3 Secondary persona: on-call engineer

**Context:** Responds to a landing-rate or confirmation-latency alert.

**Goals:**

- determine whether the change is isolated or systemic;
- find affected transaction types and routes;
- distinguish on-chain failure from absence before inclusion;
- share a stable incident link or report.

**Success statement:** “Within minutes I can identify the failure category, uncertainty, and scope.”

### 6.4 Blocking persona: security reviewer

**Context:** Evaluates whether transaction telemetry can leave the application environment.

**Goals:**

- verify that keys and credentials cannot be captured;
- control address, signature, and business-identifier exposure;
- enforce retention and deployment policies;
- inspect the code and data schema.

**Success statement:** “I understand exactly what is collected and can select an acceptable privacy mode.”

## 7. Jobs to be done

### JTBD-1: Investigate a missing transaction

When an application reports a submitted transaction but no result is visible, help an engineer reconstruct the transaction's lifecycle so the engineer can distinguish known facts from likely explanations and decide whether retrying is safe.

### JTBD-2: Investigate an executed failure

When a transaction lands but fails, show simulation and on-chain error evidence, instruction context, compute information, and related attempts so the engineer can locate the failing stage quickly.

### JTBD-3: Detect a reliability regression

When a release, route, or network condition changes, compare landing, latency, cost, and diagnostic coverage against a baseline so the team can detect and reverse harmful changes.

### JTBD-4: Evaluate submission strategies

When a team considers a different fee, endpoint, sender, or retry strategy, measure outcomes with consistent definitions so the decision is not based on provider marketing or isolated anecdotes.

### JTBD-5: Produce an incident report

When a technical or business stakeholder asks what happened, produce a sanitized report containing the timeline, evidence, impact, limitations, and recommended next actions.

### JTBD-6: Improve observability coverage

When Landfall cannot determine what happened, identify which event or field was missing so the team can improve instrumentation before the next incident.

## 8. Representative use cases

### UC-1: Accepted by RPC, never observed on-chain

An application receives a signature or successful JSON-RPC response. The signature never appears through the configured status observers before `lastValidBlockHeight` passes. Landfall shows submission attempts, route responses, block-height observations, and missing evidence. It classifies the lifecycle as expired without observed inclusion and does not claim a single confirmed network cause.

### UC-2: Simulation succeeds, on-chain execution fails

Landfall correlates the simulation result with the included transaction and displays the on-chain `InstructionError`. It labels execution failure as confirmed, highlights that state changed between simulation and execution as a possible explanation only when evidence supports that distinction, and avoids calling the event a dropped transaction.

### UC-3: Compute limit is insufficient

Simulation or on-chain metadata reports a compute-budget-related error. Landfall displays requested units, available consumed units, and a proposed headroom value within configured safety limits. The recommendation remains advisory.

### UC-4: Confirmation appears to time out but transaction succeeds

The application records a client timeout, then a later status observer sees processed/confirmed execution. Landfall labels the business-facing timeout separately from network success and warns against creating a duplicate business action without checking idempotency.

### UC-5: Release regression

A new release changes blockhash acquisition or retry behavior. The dashboard shows a statistically meaningful rise in expiration or ambiguous attempts for the affected flow and version.

### UC-6: Route comparison

A team uses two routes. Landfall compares client-observed acceptance latency, status-observation latency, landing rate, and data coverage. It explicitly states that when identical bytes are submitted to multiple routes, it may be impossible to attribute final inclusion to one route without route-specific receipts.

### UC-7: Jito bundle accepted but not landed

When the specialized adapter is enabled, Landfall correlates bundle identifiers, inflight and terminal bundle status, transaction signatures, tips, and on-chain observations. Acceptance of a bundle ID is not displayed as successful landing.

## 9. Terminology and semantic model

Consistent language is essential because “failed transaction” is ambiguous.

### 9.1 Core entities

**Business action:** The customer's intended operation, such as one swap, payout, or position update. A business action may produce multiple distinct signed transactions.

**Transaction trace:** Landfall's correlation object for one signed transaction message and its lifecycle.

**Submission attempt:** One attempt to submit the same signed transaction bytes through a route. Multiple submission attempts may share a signature.

**Replacement transaction:** A new signed transaction for the same business action, usually with changed blockhash, fee, instructions, or signature.

**Route:** A configured JSON-RPC endpoint or specialized submission service. Secrets are stored separately from the route display name.

**Landing:** First reliable observation that the transaction was included and processed in a slot. “Landed” does not mean execution succeeded.

**Execution success:** The landed transaction has no on-chain execution error.

**Execution failure:** The landed transaction contains an on-chain execution error.

**Confirmation:** Observation at a specified Solana commitment level. The UI must always name the commitment rather than use “confirmed” generically.

**Expired without observed inclusion:** The last valid block height has passed and no configured observer has found the transaction. This is an observation boundary, not proof about every fork or unobserved system.

**Diagnostic coverage:** Percentage of eligible traces with enough required evidence to reach a configured classification level.

### 9.2 Diagnostic certainty

**Confirmed:** Direct application, RPC, simulation, or on-chain evidence supports the classification under documented rules.

**Probable:** Multiple signals support the classification, but another plausible explanation remains.

**Unknown:** Evidence is missing, contradictory, or insufficient. The product identifies the missing data when possible.

Certainty applies to a specific claim. A trace can have a confirmed terminal state and an unknown underlying network cause.

## 10. Success metrics

### 10.1 Customer outcome metrics

- time to diagnose a representative incident;
- percentage of incidents with an actionable conclusion;
- landing rate;
- execution success rate;
- end-to-end transaction success rate;
- p50/p95 time from first submission to processed;
- p50/p95 time from first submission to confirmed;
- expiration-without-observed-inclusion rate;
- cost per successfully executed transaction;
- submission attempts per unique signed transaction;
- replacement transactions per business action;
- ambiguous or potentially unsafe retry rate;
- diagnostic coverage.

### 10.2 Product activation metrics

- time from installation start to first complete trace;
- percentage of installations producing all required P0 events;
- number of active transaction flows instrumented;
- number of traces opened during a real investigation;
- report exports;
- recommendations reviewed and marked accepted/rejected.

### 10.3 Commercial validation metrics

- qualified interviews;
- teams supplying real data;
- staging and production installations;
- paid pilots;
- pilot-to-continuation conversion;
- recurring revenue;
- referral rate.

### 10.4 Metric definitions

#### Eligible signed transactions

Unique signed transaction traces for which instrumentation was enabled and a submission was intended. Client-cancelled signing requests and deliberately sampled-out traces are excluded.

#### Landing rate

```text
unique eligible signed transactions observed processed
------------------------------------------------------
unique eligible signed transactions with a completed observation window
```

Submission retries of identical signed bytes do not increase the denominator. Replacement transactions do increase the transaction denominator but remain grouped under the same business action when correlation is provided.

#### Execution success rate

```text
landed transactions without an on-chain execution error
--------------------------------------------------------
all landed transactions with a known execution result
```

#### End-to-end success rate

```text
unique eligible signed transactions executed successfully
----------------------------------------------------------
unique eligible signed transactions with a completed observation window
```

#### Cost per successful execution

Sum of observable transaction fees, priority fees, and configured specialized tips divided by successful executions. The UI must state which cost components are available and exclude unknown components rather than treating them as zero.

### 10.5 MVP target values

These targets evaluate product quality, not customer-network performance:

- first complete trace in under 30 minutes for a documented Node.js server integration;
- at least 95% of correctly instrumented traces have internally consistent lifecycle states;
- 100% of diagnoses display evidence and certainty;
- 0 private-key or credential fields persisted in automated security fixtures;
- report generation in under 10 seconds for 100,000 traces;
- dashboard summary p95 response under 2 seconds for a 24-hour view containing 1 million events on reference hardware;
- telemetry outage does not cause a submitted customer transaction to fail.

## 11. Release scope and priorities

### 11.1 R0 — Validation prototype

Purpose: support concierge diagnostics, not public production use.

Includes:

- documented NDJSON/JSON event schema;
- Rust CLI to ingest a file and generate a report;
- synthetic and controlled Solana fixtures;
- lifecycle state engine;
- initial diagnostic rules;
- HTML or Markdown report;
- no dashboard, authentication, or continuous collector required.

Exit condition: Gate B from the validation strategy passes or produces a clear narrower scope.

### 11.2 R1 — Open-source self-hosted MVP (P0)

Includes:

- TypeScript server-side SDK;
- standard Solana JSON-RPC instrumentation;
- Rust collector API;
- PostgreSQL storage;
- background status observer;
- lifecycle engine and diagnostic rules;
- single-team local dashboard;
- project, environment, flow, version, and route dimensions;
- privacy modes and redaction;
- Docker Compose deployment;
- CLI health check and report export;
- demo application and reproducible fixtures.

### 11.3 R1.1 — Pilot-driven extensions (P1)

Candidates, implemented only when required by a paid or strongly validated pilot:

- Jito transaction and bundle adapter;
- multiple observer RPCs;
- websocket observation;
- route regression alerts;
- OpenTelemetry exporter;
- durable local spool;
- browser/wallet adapter;
- Rust application SDK;
- scheduled report delivery;
- self-hosted production hardening.

### 11.4 R2 — Hosted team product

Conditional on H6 recurring-value validation.

Includes:

- multi-tenant ingestion;
- organizations, projects, environments, and API keys;
- team roles;
- metering and plan enforcement;
- hosted retention;
- alerts and integrations;
- billing;
- tenant isolation and compliance controls;
- hosted status-observer fleet.

## 12. P0 user experience

### 12.1 Installation flow

1. User clones or installs Landfall.
2. User starts the local stack with Docker Compose.
3. User creates a project and environment in the CLI or dashboard.
4. Landfall generates a local ingestion token.
5. User installs the TypeScript package.
6. User wraps a supported transaction flow or emits explicit lifecycle events.
7. User sends a controlled devnet or staging transaction.
8. The CLI health check verifies event completeness and clock behavior.
9. The dashboard shows the first trace and explains any missing instrumentation.

### 12.2 Overview dashboard

The overview answers:

- Are transaction outcomes healthy now?
- Did a metric change compared with the selected baseline?
- Which flows, routes, or versions are affected?
- Is diagnostic coverage sufficient?

Required panels:

- eligible transactions;
- landing rate;
- execution success rate;
- p50/p95 time-to-processed and time-to-confirmed;
- expiration-without-observed-inclusion rate;
- unknown-diagnosis rate;
- cost per successful execution when cost data is complete;
- trend by time bucket;
- breakdown by flow, route, app version, and terminal state;
- data-quality warnings.

### 12.3 Trace search

Users can search or filter by:

- signature;
- Landfall trace ID;
- customer-supplied business-action correlation ID;
- environment;
- flow;
- route;
- app version;
- time range;
- lifecycle state;
- execution result;
- diagnosis category;
- certainty;
- data-completeness warning.

Business-action IDs may be hashed or disabled under privacy policy. Search behavior must make privacy limitations clear.

### 12.4 Trace detail

The trace detail page contains:

1. Current state and execution result.
2. Diagnostic summary and certainty.
3. Evidence-linked lifecycle timeline.
4. Transaction configuration at signing/submission.
5. Simulation result.
6. Submission attempts grouped by route.
7. Status observations by source and commitment.
8. Expiration clock and observation window.
9. Fees and compute information.
10. Related replacement transactions under the same business action.
11. Recommendations and their evidence.
12. Missing-data checklist.
13. Sanitized export action.

The page must distinguish “RPC accepted request,” “signature returned,” “observed processed,” “execution succeeded,” and “confirmed/finalized.” These states must never be collapsed into one success icon.

### 12.5 Comparison view

The MVP can compare two cohorts using:

- time windows;
- application versions;
- transaction flows;
- route labels;
- fee-policy labels;
- compute-policy labels.

It shows sample size, metric definition, missing-data rate, absolute change, and relative change. Statistical significance is P1; P0 must at least prevent comparisons with insufficient completed observation windows and must show a warning for small samples.

### 12.6 Report export

Users can export a self-contained HTML and JSON report containing:

- report metadata and filters;
- metric definitions;
- cohort sizes;
- lifecycle distributions;
- selected trace timelines;
- diagnoses and evidence;
- accepted/rejected recommendations;
- data limitations;
- redaction mode;
- Landfall version and rule-set version.

Markdown export is desirable for GitHub issues and audit reports. PDF is not required in P0.

## 13. Functional requirements

Priority labels:

- **P0:** required for open-source MVP;
- **P1:** pilot-driven next release;
- **P2:** hosted/future.

### 13.1 Project and configuration

#### FR-CONFIG-001 — Project identity (P0)

The system shall support a project with one or more named environments.

Acceptance criteria:

- events without a valid project/environment identity are rejected with a structured error;
- project display name can change without changing stored identity;
- `production`, `staging`, `devnet`, and custom environment names are supported;
- environment is present on every trace.

#### FR-CONFIG-002 — Transaction-flow labels (P0)

The SDK shall allow the application to assign a low-cardinality flow name such as `swap`, `payout`, or `rebalance`.

Acceptance criteria:

- flow is optional but missing flow generates a data-quality recommendation;
- high-cardinality values are detected and warned about;
- flow names can be filtered and compared.

#### FR-CONFIG-003 — Route registry (P0)

The system shall map secret-bearing endpoint configuration to a non-secret route identifier and display name.

Acceptance criteria:

- stored trace events never contain credential-bearing endpoint URLs by default;
- URL query parameters, user info, and configured secret path segments are redacted;
- the UI displays route labels, not secrets;
- a route can be disabled without deleting historical data.

#### FR-CONFIG-004 — Version and policy labels (P0)

Events shall support application version, SDK version, Landfall rule-set version, fee-policy label, compute-policy label, and retry-policy label.

Acceptance criteria:

- labels appear in trace detail and cohort filters;
- missing application version generates a warning in production environments;
- arbitrary high-cardinality labels are limited by configuration.

### 13.2 Instrumentation SDK

#### FR-SDK-001 — Explicit transaction context (P0)

The SDK shall create a `TraceContext` before submission and allow optional association with a business action.

Required fields:

- random Landfall trace ID;
- project and environment;
- transaction flow;
- application version when configured;
- creation wall-clock timestamp;
- monotonic elapsed-time origin;
- privacy mode.

Acceptance criteria:

- trace IDs are globally unique UUIDv7 or equivalently sortable random identifiers;
- the SDK does not derive trace identity only from signature;
- one business action can link multiple replacement traces;
- the same signed bytes submitted more than once remain one trace with multiple submission attempts.

#### FR-SDK-002 — Supported Node.js clients (P0)

The initial SDK shall support server-side Node.js 24 LTS and explicit adapters for the selected Solana client library.

The first portfolio adapter targets the exact `@solana/kit` 8.2.0 lane frozen in
the [P0 support matrix](support-matrix.md). A later `@solana/web3.js` v3 spike
and, if customer evidence warrants it, a separate legacy v1 adapter can be
prioritized without changing the neutral event protocol. `@solana/web3-compat`
is not a support roadmap target.

Acceptance criteria:

- the supported versions are documented and tested;
- unsupported versions fail with an actionable message;
- users can emit the neutral event schema manually when an adapter is unavailable.

#### FR-SDK-003 — Lifecycle event capture (P0)

The SDK shall capture or accept explicit events for:

- transaction preparation;
- recent blockhash acquisition;
- simulation start/result;
- signing start/result when observable;
- submission start/result;
- retry/rebroadcast;
- application confirmation wait start/result;
- business outcome when supplied.

Acceptance criteria:

- every event has a unique event ID and timestamp;
- event ingestion is idempotent;
- capture works when preflight is skipped;
- unavailable events are shown as missing, not synthesized.

#### FR-SDK-004 — Transaction fingerprint (P0)

The SDK shall calculate a cryptographic digest of the canonical signed bytes when available without persisting the bytes by default.

Acceptance criteria:

- identical bytes produce the same digest within a deployment;
- digest algorithm and encoding are versioned;
- raw signed bytes are disabled by default;
- fingerprinting does not log signing material;
- replacement transactions receive different trace identities even when linked to one business action.

#### FR-SDK-005 — Non-blocking telemetry (P0)

Telemetry must not be on the critical path of transaction submission.

Acceptance criteria:

- collector unavailability does not prevent or change submission;
- SDK emission errors are available through a callback/metric but are not thrown from the transaction call by default;
- a configurable asynchronous buffer is used;
- buffer overflow follows a documented drop policy and increments a counter;
- strict fail-closed telemetry is not supported in P0.

#### FR-SDK-006 — Sensitive-data filtering (P0)

The SDK shall inspect structured fields and remove prohibited data before emission.

Prohibited by default:

- private keys and seed phrases;
- signer secret material;
- authorization headers;
- cookies;
- complete credential-bearing RPC URLs;
- arbitrary application request bodies;
- raw signed transactions;
- full stack-local environment variables.

Acceptance criteria:

- automated fixtures containing known secret patterns are redacted;
- event metadata uses an allowlist rather than unrestricted object serialization;
- custom metadata has size and key restrictions;
- redaction failures can be tested locally before production use.

#### FR-SDK-007 — Sampling (P1)

The SDK may support deterministic sampling for successful traces while retaining all configured error/expiration traces.

Sampling must not be implemented until metric correction rules and pilot volume require it.

### 13.3 Privacy modes

#### FR-PRIV-001 — Standard mode (P0)

Standard mode shall store transaction signature, transaction digest, route label, configuration, and structured lifecycle events, but no raw signed transaction or arbitrary account list by default.

#### FR-PRIV-002 — Full diagnostic mode (P0)

Full diagnostic mode may store explicitly approved public account addresses and instruction/program identifiers needed for deeper analysis.

Acceptance criteria:

- enablement is explicit per environment;
- UI displays a persistent privacy warning;
- export can re-redact these fields;
- raw transaction storage remains a separate opt-in.

#### FR-PRIV-003 — Strict mode (P1)

Strict mode shall allow the signature and public addresses to remain in the application environment. The SDK or local sidecar must perform status observation and emit only pseudonymous structured results.

The UI must explain that central re-query and retrospective enrichment are unavailable in strict mode.

### 13.4 Collector and ingestion

#### FR-INGEST-001 — Batch ingestion API (P0)

The Rust collector shall expose a versioned authenticated endpoint for batches of events.

Acceptance criteria:

- accepts compressed JSON batches;
- validates schema version and size limits;
- returns per-batch acceptance and structured rejection reasons;
- uses event IDs for idempotency;
- rejects prohibited fields detected at the collector boundary;
- records collector receive time separately from source event time.

#### FR-INGEST-002 — Clock-quality analysis (P0)

The collector shall estimate source clock offset and preserve monotonic durations reported by the SDK.

Acceptance criteria:

- impossible event ordering is flagged;
- wall-clock skew above a configurable threshold creates a data-quality warning;
- latency calculations prefer monotonic duration for same-process spans;
- cross-system latency identifies clock limitations.

#### FR-INGEST-003 — Event ordering and idempotency (P0)

The collector shall accept duplicate and out-of-order events and derive the same final state after reprocessing.

Acceptance criteria:

- duplicate event IDs do not duplicate metrics;
- late terminal events can update a previously expired/unknown display while preserving history;
- lifecycle derivation is repeatable for the same ordered event set and rule version;
- conflicting immutable facts are flagged rather than silently overwritten.

#### FR-INGEST-004 — Local durable buffer (P1)

The SDK or sidecar may persist encrypted or access-restricted event batches during collector outage. This is excluded from P0 unless required by a pilot.

### 13.5 Status observation

#### FR-OBS-001 — Signature status polling (P0)

The observer shall query configured Solana JSON-RPC sources for eligible signatures until a terminal observation policy is met.

Acceptance criteria:

- polling cadence and commitment are configurable within safe bounds;
- observer sources are labeled separately from submission routes;
- null status is recorded as an observation, not as proof of non-delivery;
- status changes are preserved;
- polling stops according to documented finalization, expiration, retention, or privacy rules.

#### FR-OBS-002 — Block-height tracking (P0)

The observer shall track block height required to evaluate `lastValidBlockHeight`.

Acceptance criteria:

- block height source and commitment are recorded;
- expiration is not based solely on elapsed wall-clock time;
- missing `lastValidBlockHeight` reduces diagnostic certainty;
- observer lag or source disagreement is represented as a warning.

#### FR-OBS-003 — Transaction enrichment (P0)

When a transaction is observed on-chain and privacy policy permits, the observer shall fetch execution metadata.

Expected data:

- slot;
- block time when available;
- execution error;
- fee;
- compute units consumed when available;
- log-message presence and optionally redacted logs;
- confirmation status.

Acceptance criteria:

- missing optional fields are represented as unavailable;
- full logs are not collected by default;
- RPC errors do not overwrite a previously valid observation;
- enrichment source and time are recorded.

#### FR-OBS-004 — Multiple observer sources (P1)

The product shall support querying more than one observer to identify source lag and increase observation coverage. P0 data models must allow multiple sources even if the UI initially configures one.

#### FR-OBS-005 — Websocket observation (P1)

Websocket subscriptions may reduce observation latency, with polling retained as the correctness fallback.

#### FR-OBS-006 — Reorg and commitment evolution (P0)

The lifecycle engine shall preserve transitions between processed, confirmed, finalized, and disappeared observations.

Acceptance criteria:

- `processed` is not treated as irreversible;
- a later conflicting observation is shown in history;
- final metrics use the configured measurement commitment;
- reorg-like disappearance is classified separately from ordinary expiration when evidence exists.

### 13.6 Lifecycle engine

#### FR-LIFE-001 — State derivation (P0)

The engine shall derive a current lifecycle state from immutable events rather than allowing clients to set a final status directly.

Canonical states:

- `created`;
- `simulated_ok`;
- `simulation_failed`;
- `signing_pending`;
- `signed`;
- `submission_pending`;
- `submission_rejected`;
- `submitted`;
- `observed_processed`;
- `observed_confirmed`;
- `observed_finalized`;
- `execution_failed_processed`;
- `execution_failed_confirmed`;
- `execution_failed_finalized`;
- `expired_without_observed_inclusion`;
- `client_abandoned`;
- `observation_incomplete`;
- `conflicting_evidence`.

Some states describe stage while execution result is a separate dimension. The implementation may use orthogonal state fields internally if that prevents invalid combinations.

#### FR-LIFE-002 — Business-action grouping (P0)

The engine shall group multiple transaction traces under a customer-supplied or generated business-action correlation ID.

Acceptance criteria:

- one action can contain retries of identical bytes and replacement transactions;
- the UI warns when more than one distinct transaction for the same action executes successfully;
- the product does not claim a duplicate business effect without application-provided outcome evidence;
- grouping can be disabled under privacy policy.

#### FR-LIFE-003 — Observation window (P0)

Every metric requiring a terminal result shall use a completed observation window.

Acceptance criteria:

- recent in-flight traces are excluded from terminal-rate denominators;
- the UI shows the configured window and excluded count;
- expiration uses block height when available;
- durable-nonce transactions are excluded from blockhash-expiration rules unless explicitly supported.

#### FR-LIFE-004 — Durable nonce handling (P1)

P0 shall detect likely durable-nonce transactions when data permits and label their expiration analysis unsupported rather than applying recent-blockhash rules.

### 13.7 Diagnostic engine

#### FR-DIAG-001 — Versioned deterministic rules (P0)

Diagnostic classifications shall be produced by versioned deterministic rules.

Acceptance criteria:

- every diagnosis records rule ID and rule-set version;
- rerunning a historical trace under a newer rule set does not erase the original diagnosis;
- rule inputs and supporting events are viewable;
- no LLM output can set or override authoritative certainty.

#### FR-DIAG-002 — Confirmed classifications (P0)

Initial confirmed categories include, where direct evidence exists:

- client construction/configuration error;
- simulation execution error;
- simulation blockhash-not-found response;
- signing rejected or failed as reported by application integration;
- RPC submission rejection with structured error;
- on-chain instruction/program error;
- on-chain compute-budget-related failure;
- insufficient funds/fee error when directly reported;
- duplicate signature/already processed response when directly reported;
- blockhash validity window passed without observed inclusion under the configured observer policy;
- client confirmation timeout followed by observed network execution;
- conflicting source observations.

“Confirmed” applies to the observed category, not to an unobserved validator-internal cause.

#### FR-DIAG-003 — Probable classifications (P0)

Initial probable categories may include:

- uncompetitive priority fee for observed comparable local contention;
- route/RPC degradation based on aggregate response errors or latency;
- commitment mismatch or observer lag;
- transaction likely not propagated before expiration;
- insufficient compute headroom when simulation consumption approaches the requested limit but direct failure evidence is absent;
- excessive signing delay consuming the validity window;
- unsafe or redundant retry pattern.

Acceptance criteria:

- every probable rule names at least one plausible alternative explanation;
- UI language uses “likely,” “consistent with,” or equivalent—not “caused by”;
- thresholds are configurable and documented;
- a probable category cannot be promoted to confirmed by repetition alone.

#### FR-DIAG-004 — Unknown and missing evidence (P0)

When classification is not defensible, Landfall shall display `unknown` and list missing evidence.

Possible missing items:

- no `lastValidBlockHeight`;
- no stable trace ID;
- no submission response;
- no route label;
- no simulation data;
- no observer coverage through the validity window;
- clock skew;
- signature not stored in strict mode;
- durable nonce unsupported;
- same business action not correlated;
- fee or compute configuration unavailable.

#### FR-DIAG-005 — Data-quality score (P0)

Each trace shall receive a data-quality assessment independent of transaction success.

Acceptance criteria:

- score or grade is derived from documented required and optional fields;
- missing critical fields are shown individually;
- a high score does not imply transaction success;
- aggregate diagnostic coverage can be filtered by SDK/app version.

### 13.8 Recommendation engine

#### FR-REC-001 — Advisory-only behavior (P0)

Recommendations shall not automatically change, sign, submit, or retry a transaction.

#### FR-REC-002 — Blockhash recommendations (P0)

Rules may recommend:

- acquiring a blockhash closer to signing/submission;
- storing `lastValidBlockHeight`;
- aligning blockhash and preflight commitment;
- avoiding inappropriate blockhash reuse;
- measuring signing delay;
- using a supported durable-nonce flow when validity-window constraints are inherent.

Every recommendation must reference the captured configuration and observed timing.

#### FR-REC-003 — Compute recommendations (P0)

When valid simulation consumption is available, the engine may recommend a compute limit using a configurable headroom policy.

Default proposal for evaluation:

```text
recommended_limit = min(protocol_max, ceil(simulated_units × headroom_factor))
```

The initial headroom factor may be 1.10 for experiments but must be configurable and presented as a policy, not a universal constant.

Acceptance criteria:

- no recommendation when simulation is invalid or unrepresentative without a warning;
- protocol maximum and application cap are enforced;
- requested versus consumed versus recommended units are displayed;
- recommendation explains that state-dependent execution may differ from simulation.

#### FR-REC-004 — Fee recommendations (P0 limited)

P0 may assess fee-policy evidence but shall not present a universal exact fee without a supported data source and relevant account context.

Permitted recommendations:

- record the fee policy and actual requested fee;
- use a current local/account-aware estimate when available;
- apply customer-defined maximum cost;
- compare cohorts by cost per successful execution;
- avoid interpreting a global fee percentile as sufficient for every transaction.

Exact automated fee estimation is P1 and requires a provider-neutral methodology or clearly labeled provider-specific adapter.

#### FR-REC-005 — Retry recommendations (P0)

Rules may recommend:

- poll through `lastValidBlockHeight` rather than a fixed short timeout;
- distinguish rebroadcast of identical signed bytes from replacement transaction creation;
- check status before creating a replacement;
- attach a business-action correlation ID;
- implement application-level idempotency for non-idempotent effects;
- record every route and retry timestamp.

The product must never tell a user that a replacement is universally safe based only on absence from one RPC source.

#### FR-REC-006 — Recommendation feedback (P0)

Users shall mark recommendations as accepted, rejected, implemented, or not applicable and optionally record a reason.

This feedback supports pilot reporting but does not automatically train a model.

### 13.9 Metrics and comparisons

#### FR-METRIC-001 — Stable metric semantics (P0)

Every metric in the UI and exports shall have a definition accessible from the display.

Acceptance criteria:

- retries of identical bytes are not counted as separate landed transactions;
- in-flight traces are excluded from completed-window denominators;
- sampled data is labeled and corrected only through documented rules;
- unavailable fee components are not treated as zero;
- commitment used for latency is displayed.

#### FR-METRIC-002 — Cohort dimensions (P0)

Metrics shall support the following dimensions:

- time;
- environment;
- flow;
- application version;
- SDK version;
- route;
- fee-policy label;
- compute-policy label;
- retry-policy label;
- terminal state;
- execution result;
- diagnostic category/certainty;
- data-quality grade.

#### FR-METRIC-003 — Minimum sample warning (P0)

The UI shall warn when a comparison cohort contains fewer than a configurable minimum number of completed traces.

P0 does not claim statistical significance. P1 may add confidence intervals and experiment design.

#### FR-METRIC-004 — Route attribution limitation (P0)

When identical transaction bytes are sent through multiple routes, the system shall not attribute inclusion to the last or fastest route without route-specific proof.

The product may compare route acceptance latency and error rates, while landing attribution is labeled unknown or shared.

### 13.10 Dashboard and reporting

#### FR-UI-001 — Overview (P0)

The UI shall implement the overview described in Section 12.2 with selectable time window and baseline.

#### FR-UI-002 — Trace list and detail (P0)

The UI shall implement search, filters, evidence timeline, diagnosis, data quality, related traces, and sanitized export.

#### FR-UI-003 — Accessible state language (P0)

Status must never be represented by color alone. Labels and icons must distinguish:

- submitted;
- landed and successful;
- landed and failed;
- expired without observed inclusion;
- incomplete observation;
- conflicting evidence.

#### FR-UI-004 — Comparison view (P0)

The UI shall compare two cohorts and expose sample size, observation completeness, and metric definitions.

#### FR-REPORT-001 — CLI health check (P0)

The CLI shall verify:

- collector reachability;
- authentication;
- schema compatibility;
- route redaction;
- clock skew;
- required event coverage from a recent trace;
- status-observer connectivity;
- privacy mode.

#### FR-REPORT-002 — Portable report (P0)

The CLI/backend shall export self-contained HTML and structured JSON. Export redaction shall be selectable and recorded in report metadata.

### 13.11 Alerts

#### FR-ALERT-001 — Local threshold alerts (P1)

After validation, the product may support alerts for:

- landing-rate regression;
- expiration increase;
- route error/latency increase;
- diagnostic coverage decrease;
- execution-error increase;
- ambiguous retry detection.

Alerting requires completed-window logic, minimum volume, deduplication, recovery notifications, and an attached cohort link. It is excluded from P0 to avoid premature operational complexity.

### 13.12 Administration

#### FR-ADMIN-001 — Retention (P0)

Self-hosted deployments shall support configurable event and trace retention.

Acceptance criteria:

- deletion jobs are observable and retryable;
- aggregate metrics derived from deleted traces follow documented policy;
- default retention is documented;
- one trace can be deleted by identifier for pilot privacy requests.

#### FR-ADMIN-002 — Rule management (P0)

Rule thresholds may be configured in version-controlled configuration. Arbitrary executable user rules are out of scope.

#### FR-ADMIN-003 — Multi-user RBAC (P2)

The local MVP assumes a trusted single team and deployment boundary. Hosted RBAC is not part of P0.

## 14. Lifecycle state model

### 14.1 Conceptual state flow

```text
created
  ├─> simulation_failed
  └─> simulated_ok / simulation_skipped
          └─> signed
                 ├─> submission_rejected
                 └─> submitted
                        ├─> observed_processed
                        │      ├─> execution_success
                        │      └─> execution_failure
                        │             └─> confirmed/finalized evolution
                        ├─> expired_without_observed_inclusion
                        ├─> observation_incomplete
                        └─> conflicting_evidence
```

Submission attempts and status observations are append-only child records. The current state is a projection.

### 14.2 Important edge behavior

- An RPC error does not prove the transaction was not forwarded.
- An RPC success response does not prove inclusion.
- A signature does not prove inclusion.
- `processed` does not prove finality.
- A landed transaction can execute unsuccessfully.
- A client timeout can coexist with network success.
- Passing `lastValidBlockHeight` without an observation supports an expired-without-observed-inclusion classification, not a universal proof of network non-receipt.
- A later valid observation may supersede a previous local conclusion while preserving the earlier timeline.
- Durable-nonce transactions require a different validity model.
- A bundle identifier represents acceptance by a bundle endpoint, not inclusion.

## 15. Diagnostic taxonomy

### 15.1 Stage: preparation

- invalid transaction construction;
- missing required account/signature;
- transaction size or version incompatibility;
- blockhash acquisition failure;
- unsupported durable nonce;
- configuration/data missing.

### 15.2 Stage: simulation

- simulation succeeded;
- blockhash not found;
- instruction/program error;
- compute-budget failure;
- insufficient funds/fees;
- node simulation error;
- simulation skipped;
- simulation result unavailable.

### 15.3 Stage: signing

- user/signer rejected;
- signer timeout;
- remote signer error;
- excessive signing delay;
- signature complete;
- signing telemetry unavailable.

### 15.4 Stage: submission

- route accepted request;
- structured RPC rejection;
- transport timeout;
- connection/TLS/DNS failure;
- rate limit;
- authentication/authorization failure;
- blockhash rejected;
- duplicate/already processed;
- response malformed;
- outcome ambiguous.

### 15.5 Stage: inclusion and validity

- observed processed;
- observed confirmed;
- observed finalized;
- expired without observed inclusion;
- observer incomplete;
- observer disagreement;
- processed observation disappeared;
- likely uncompetitive fee;
- likely propagation/routing problem;
- cause unknown.

### 15.6 Stage: execution

- success;
- instruction/program error;
- compute-budget failure;
- account lock/contention-related error when explicitly reported;
- insufficient funds;
- slippage/application constraint;
- custom program error decoded when a trusted decoder is configured;
- undecoded execution error.

### 15.7 Stage: business reconciliation

- business outcome confirmed by application;
- network succeeded, client reported timeout;
- multiple successful replacement transactions detected;
- business idempotency unknown;
- reconciliation event missing.

## 16. Data model

The database design may evolve, but P0 must preserve the following logical entities.

### 16.1 Project

- `id`;
- `display_name`;
- `created_at`;
- `default_retention_days`;
- `privacy_policy_version`.

### 16.2 Environment

- `id`;
- `project_id`;
- `name`;
- `cluster`;
- `privacy_mode`;
- `created_at`;
- `disabled_at`.

### 16.3 Route

- `id`;
- `environment_id`;
- `display_name`;
- `route_type`;
- `region` when configured;
- non-secret endpoint fingerprint;
- secret reference, stored outside event records;
- enabled state.

### 16.4 BusinessAction

- `id`;
- `environment_id`;
- optional pseudonymous external correlation digest;
- `flow`;
- `created_at`;
- optional business outcome and timestamp;
- privacy/redaction metadata.

### 16.5 TransactionTrace

- `id`;
- optional `business_action_id`;
- signature according to privacy mode;
- signed-bytes digest;
- message/version type;
- recent blockhash or approved digest;
- `last_valid_block_height`;
- fee payer/address fields according to privacy mode;
- flow and application version;
- created/signed timestamps;
- derived lifecycle state;
- derived execution result;
- observation completeness;
- data-quality grade;
- rule-set version;
- first and last event time.

### 16.6 Simulation

- `trace_id`;
- request timestamp;
- source route;
- commitment/configuration;
- result/error category;
- units consumed;
- replacement blockhash behavior if used;
- logs-present flag;
- optional redacted log reference.

### 16.7 SubmissionAttempt

- `id`;
- `trace_id`;
- route ID;
- attempt sequence;
- start/end timestamps and monotonic duration;
- encoding;
- preflight settings;
- `maxRetries` or equivalent;
- transport result;
- RPC result/error category;
- returned signature/digest;
- specialized route receipt or bundle ID;
- redaction metadata.

### 16.8 StatusObservation

- `id`;
- `trace_id`;
- observer source;
- observed time;
- commitment/status;
- slot;
- error presence/category;
- block height at observation;
- response latency;
- source result quality.

### 16.9 ExecutionMetadata

- `trace_id`;
- source;
- slot;
- block time;
- fee components available;
- compute units consumed;
- execution error category and structured value;
- log availability;
- finality evolution.

### 16.10 Diagnostic

- `id`;
- `trace_id` or cohort ID;
- category;
- claim text key;
- certainty;
- rule ID and rule-set version;
- supporting event IDs;
- alternative explanation keys;
- created time;
- superseded-by relation.

### 16.11 Recommendation

- `id`;
- trace/cohort scope;
- category;
- severity/priority;
- rule and evidence IDs;
- structured proposed value when applicable;
- limitation text key;
- user disposition;
- disposition reason and time.

### 16.12 RawEvent

- immutable event ID;
- schema version;
- project/environment/trace IDs;
- event type;
- source timestamp;
- monotonic offset/duration fields;
- collector receive timestamp;
- SDK/source version;
- allowlisted structured payload;
- redaction version;
- integrity metadata.

Raw events are retained long enough for reproducibility according to policy. Derived projections can be rebuilt from them.

## 17. Event schema

### 17.1 Envelope

Illustrative shape:

```json
{
  "schema_version": "1.0",
  "event_id": "0198...",
  "event_type": "solana.submission.completed",
  "occurred_at": "2026-08-29T12:00:00.123Z",
  "monotonic_ns": 28400123,
  "project_id": "project_local",
  "environment": "staging",
  "trace_id": "0198...",
  "business_action_id": "optional-pseudonymous-id",
  "source": {
    "sdk": "landfall-js",
    "version": "0.1.0",
    "service": "swap-worker",
    "app_version": "git-sha"
  },
  "attributes": {}
}
```

### 17.2 P0 event types

- `solana.trace.created`;
- `solana.blockhash.acquired`;
- `solana.simulation.started`;
- `solana.simulation.completed`;
- `solana.signing.started`;
- `solana.signing.completed`;
- `solana.submission.started`;
- `solana.submission.completed`;
- `solana.submission.retry_scheduled`;
- `solana.confirmation_wait.started`;
- `solana.confirmation_wait.completed`;
- `solana.status.observed`;
- `solana.execution.enriched`;
- `solana.business_outcome.observed`;
- `landfall.data_quality.detected`;
- `landfall.diagnosis.generated`;
- `landfall.recommendation.generated`.

### 17.3 Schema rules

- Events are append-only.
- Unknown fields are rejected or stored only inside a versioned extension namespace.
- Custom attributes use an explicit allowlist and cardinality limit.
- Monetary and fee values use integers in base units with a named unit.
- Large integers are serialized as decimal strings where JSON number precision is unsafe.
- Timestamps use UTC RFC 3339 plus monotonic fields where possible.
- Error payloads use normalized category plus a bounded, redacted original code/message.
- Signature and address fields obey the environment's privacy mode.

## 18. Proposed technical architecture

### 18.1 Repository layout

```text
/
├── apps/
│   └── dashboard/              # Web UI
├── crates/
│   ├── landfall-core/          # Lifecycle, metrics, rules, domain types
│   ├── landfall-collector/     # Rust ingestion and query service
│   ├── landfall-observer/      # Solana status and enrichment workers
│   └── landfall-cli/           # Health checks, fixture ingestion, reports
├── packages/
│   └── sdk-ts/                 # Node.js instrumentation
├── examples/
│   ├── node-standard-rpc/
│   └── fixtures/
├── deployments/
│   └── docker-compose/
└── docs/
```

This is a proposed layout, not an implementation requirement if a simpler validation prototype is sufficient.

### 18.2 Rust core

Responsibilities:

- neutral domain types and schema validation;
- append-only event processing;
- lifecycle state projection;
- diagnostic and recommendation rules;
- metric aggregation semantics;
- report model generation;
- deterministic fixture tests.

The core must not depend directly on a web framework or one RPC provider.

### 18.3 Collector service

Proposed stack:

- Rust;
- Tokio async runtime;
- Axum HTTP API;
- SQLx for persistence;
- PostgreSQL;
- structured tracing for Landfall's own observability.

Responsibilities:

- authenticate ingestion;
- validate and redact event batches;
- persist immutable events;
- schedule projection and observation work;
- expose query/report APIs;
- serve readiness and health endpoints.

### 18.4 Observer worker

Responsibilities:

- poll block height and signature status;
- fetch transaction execution metadata;
- manage per-route rate limits and backoff;
- preserve source-specific observations;
- stop work according to validity and retention policies;
- never submit or mutate customer transactions.

### 18.5 TypeScript SDK

Responsibilities:

- create trace and business-action contexts;
- capture allowed pre-submission configuration;
- instrument the selected Solana client library;
- emit batches asynchronously;
- sanitize endpoints and metadata;
- expose manual event APIs;
- report SDK telemetry loss separately.

The SDK must be useful without monkey-patching global client behavior. Explicit wrappers are preferred for predictability.

### 18.6 Dashboard

Proposed implementation:

- TypeScript and React-based UI;
- a minimal server or static frontend backed by collector query APIs;
- no public internet dependency for self-hosted operation;
- accessible status semantics and responsive tables.

The exact frontend framework is an implementation decision. It must not force the Rust collector to expose database internals.

### 18.7 Storage strategy

P0 uses PostgreSQL for both events and derived projections to minimize operational scope. Partitioning and indexes should support time-window queries and trace lookup.

ClickHouse, Kafka, Redis, and a separate object store are not P0 dependencies. They may be introduced only after measured volume proves PostgreSQL insufficient.

### 18.8 Processing model

- ingestion writes immutable events transactionally;
- an internal worker or database-backed job table projects trace state;
- observation jobs are idempotent;
- diagnoses and recommendations record rule versions;
- projections can be rebuilt from retained events;
- no external message broker is required in P0.

## 19. APIs

Detailed OpenAPI definitions will be created during implementation. Required conceptual endpoints follow.

### 19.1 Ingestion

- `POST /api/v1/events:batch`;
- `POST /api/v1/health-check/events` for disposable installation checks.

### 19.2 Traces

- `GET /api/v1/traces`;
- `GET /api/v1/traces/{trace_id}`;
- `GET /api/v1/traces/by-signature/{signature}` when privacy mode permits;
- `GET /api/v1/business-actions/{id}`;
- `DELETE /api/v1/traces/{trace_id}` for authorized local deletion.

### 19.3 Metrics and comparisons

- `GET /api/v1/metrics/summary`;
- `POST /api/v1/comparisons`;
- `GET /api/v1/data-quality/summary`.

### 19.4 Recommendations

- `GET /api/v1/recommendations`;
- `POST /api/v1/recommendations/{id}/disposition`.

### 19.5 Reports

- `POST /api/v1/reports`;
- `GET /api/v1/reports/{id}`;
- `GET /api/v1/reports/{id}/download`.

### 19.6 Administration

- `GET /health/live`;
- `GET /health/ready`;
- local project/environment/route configuration endpoints;
- rule and schema version endpoint.

## 20. Security and privacy requirements

### 20.1 Threat model

Assets to protect:

- RPC credentials;
- transaction strategy and timing;
- public addresses that become sensitive through correlation;
- customer business-action identifiers;
- transaction volume and performance;
- deployment tokens;
- incident reports.

Primary threats:

- accidental secret capture by overly broad logging;
- credential leakage through endpoint URLs;
- unauthorized dashboard access;
- cross-project data exposure in future hosted mode;
- malicious high-cardinality or oversized payloads;
- dependency compromise;
- telemetry causing application latency or failure;
- report exports revealing redacted data;
- an attacker using detailed timing information against a trading strategy.

### 20.2 P0 controls

- allowlisted structured telemetry only;
- no arbitrary object or environment serialization;
- endpoint redaction in SDK and collector;
- ingestion token stored through deployment secrets, not config committed to Git;
- authentication required even for local network ingestion unless explicitly bound to loopback;
- request size and event count limits;
- rate limiting;
- parameterized database access;
- least-privilege database role;
- secure default network bindings;
- configurable address/signature privacy modes;
- export-specific redaction pass;
- dependency lockfiles and vulnerability scanning;
- documented deletion and retention;
- auditable rule and schema versions;
- no remote analytics/telemetry from the self-hosted product without explicit opt-in.

### 20.3 Secret tests

Automated tests shall insert representative:

- base58 private-key-like values;
- seed phrase patterns;
- bearer tokens;
- API-key query parameters;
- authenticated RPC URLs;
- cookies and authorization headers;
- arbitrary nested metadata.

Tests must verify rejection or redaction at both SDK and collector boundaries. Pattern detection is defense in depth; allowlisting is the primary control.

### 20.4 Data retention

Proposed self-hosted defaults:

- raw events: 14 days;
- derived traces and metrics: 30 days;
- reports: retained until user deletion;
- secrets: never stored inside trace/event tables.

Defaults must be revisited with pilot evidence. Production documentation must explain database backups and that deleting active records does not automatically delete external backups.

### 20.5 Hosted requirements (P2)

Before hosted multi-tenancy:

- tenant-isolation review;
- per-tenant encryption and authorization design;
- RBAC;
- audit log;
- abuse and volume controls;
- incident response plan;
- backup/restore tests;
- privacy policy and data-processing terms;
- threat model update;
- external security review appropriate to customer risk.

## 21. Non-functional requirements

### 21.1 Reliability

NFR-REL-001. SDK telemetry failure shall not fail or delay transaction submission beyond documented bounded local overhead.

NFR-REL-002. Ingestion and projection operations shall be idempotent.

NFR-REL-003. Observer outages shall create data-quality gaps rather than false terminal conclusions.

NFR-REL-004. Database migrations shall be reversible where practical and documented.

NFR-REL-005. The collector shall expose live and ready health checks.

### 21.2 Performance

Reference targets for P0:

- SDK synchronous p95 overhead under 5 ms excluding the customer's original RPC call;
- no synchronous network call added before transaction submission;
- ingestion batch response p95 under 250 ms at 100 events per batch on reference hardware;
- sustained collector throughput of at least 500 events/second on a documented development configuration;
- trace lookup p95 under 500 ms for one million retained events;
- overview query p95 under 2 seconds for a 24-hour window after expected indexing/aggregation;
- report export under 10 seconds for 100,000 traces.

Targets are validated through reproducible benchmarks and adjusted from pilot volume. They are not public service-level guarantees.

### 21.3 Scalability

- event tables are time-partition-ready;
- queries always include project/environment and bounded time ranges where applicable;
- high-cardinality metadata is rejected or bounded;
- background observation concurrency is configurable per endpoint;
- one deployment should support at least one million events/day before requiring architectural replacement, subject to measured event size and hardware.

### 21.4 Compatibility

- Linux amd64 and arm64 containers for self-hosted deployment;
- PostgreSQL 18.6 for the initial P0 release, with later minors admitted only after the support-matrix gates pass;
- Node.js 24 LTS for the initial SDK;
- exact `@solana/kit` 8.2.0 at launch;
- standard JSON-RPC endpoints;
- Solana mainnet, devnet, and local validator identified explicitly per environment.

### 21.5 Accessibility and usability

- status is not conveyed by color alone;
- tables support keyboard navigation to practical extent;
- all certainty labels have textual definitions;
- timestamps can show UTC and user-selected local time;
- numeric fee values show base unit and readable unit;
- empty and incomplete states explain the next instrumentation step;
- technical details are expandable rather than removed.

### 21.6 Operability

Landfall must observe itself using structured logs and metrics for:

- accepted/rejected events;
- redactions;
- SDK drops when reported;
- projection lag;
- observation queue depth;
- RPC error/latency by observer;
- database query latency;
- report jobs;
- retention jobs;
- rule failures;
- version information.

## 22. Detailed diagnostic rule examples

These examples constrain language and evidence. Final identifiers and thresholds belong in versioned rule definitions.

### RULE-EXP-001 — Validity window passed

Inputs:

- signed trace with `lastValidBlockHeight`;
- at least one submission attempt;
- observer block height greater than `lastValidBlockHeight`;
- no observed inclusion through configured observer policy.

Output:

- state: `expired_without_observed_inclusion`;
- certainty: confirmed for the local observation state;
- explanation: “The configured validity window passed without this deployment observing inclusion.”
- limitation: “This does not identify whether the transaction failed to leave the RPC, failed to reach a leader, lost scheduling competition, or was missed by an observer.”

### RULE-CU-001 — Direct compute-budget failure

Inputs:

- structured simulation or on-chain error explicitly representing compute exhaustion/budget failure.

Output:

- category: compute-budget failure;
- certainty: confirmed;
- recommendation only if representative units-consumed data exists.

### RULE-CU-002 — Low compute headroom

Inputs:

- successful simulation;
- requested compute limit;
- units consumed;
- consumption/request ratio above configured threshold;
- no direct compute failure.

Output:

- category: low compute headroom;
- certainty: probable risk, not transaction failure cause;
- recommendation: configured headroom calculation with state-difference warning.

### RULE-RPC-001 — Structured rejection

Inputs:

- RPC response with normalized error.

Output:

- category determined by normalized error mapping;
- certainty: confirmed for rejection by that route;
- limitation: if the client timed out or received a transport error after writing the request, the system must not assume no forwarding occurred.

### RULE-FEE-001 — Fee likely uncompetitive

Inputs:

- expired without observed inclusion;
- requested priority fee known;
- relevant local/account-aware fee observations available for the same time window;
- adequate comparable sample;
- requested fee materially below configured percentile/threshold;
- no direct rejection or execution failure.

Output:

- category: uncompetitive priority fee;
- certainty: probable;
- alternative explanations: route propagation, contention changes, leader behavior, observer gap;
- no claim that raising fee guarantees landing.

### RULE-RETRY-001 — Redundant rebroadcast

Inputs:

- same signed-bytes digest submitted multiple times after observed processed/confirmed status.

Output:

- category: unnecessary post-inclusion rebroadcast;
- certainty: confirmed from application behavior and observation order, subject to clock-quality warning;
- recommendation: stop retry loop on configured commitment.

### RULE-BIZ-001 — Multiple successful replacements

Inputs:

- one business action;
- two or more distinct signed transaction traces;
- both observed executed successfully.

Output:

- category: multiple network successes for one correlated business action;
- certainty: confirmed for network executions;
- limitation: business-level duplication is unknown without application outcome semantics;
- severity: high.

## 23. Testing strategy

### 23.1 Unit testing

- lifecycle transitions;
- invalid transition handling;
- diagnostic rule inputs and wording keys;
- certainty boundaries;
- metric denominators;
- retry grouping;
- privacy redaction;
- event schema evolution;
- endpoint credential removal;
- fee and large-integer arithmetic;
- report redaction.

Every diagnostic rule must have:

- positive fixture;
- near-miss negative fixture;
- missing-evidence fixture;
- contradictory-evidence fixture where relevant;
- snapshot of evidence references and certainty.

### 23.2 Property and fuzz testing

- duplicate/out-of-order events derive stable projections;
- malformed or oversized payloads do not crash collector;
- arbitrary metadata cannot bypass allowlist;
- state engine never emits impossible success/failure combinations;
- fee arithmetic does not overflow;
- event replay is deterministic for a rule-set version.

### 23.3 Integration testing

- SDK → collector → database → lifecycle → UI/API;
- collector outage with customer transaction continuing;
- event replay and migration;
- observer against local validator;
- simulated program error;
- on-chain execution error;
- successful processed/confirmed/finalized evolution;
- blockhash expiration fixture;
- RPC timeout and ambiguous response fixture;
- route redaction;
- retention/deletion;
- report generation.

### 23.4 Controlled network testing

Devnet and mainnet testing must use explicit budgets and non-speculative transactions.

Mainnet experiments shall:

- use low-value controlled transfers or purpose-built harmless instructions;
- set a total daily fee budget;
- avoid spam or artificial congestion;
- record methodology and limitations;
- avoid ranking providers from an inadequately controlled sample;
- never handle third-party funds.

### 23.5 Golden incident fixtures

The repository should include sanitized, deterministic fixtures for:

- success;
- simulation error;
- RPC rejection;
- transport timeout with later success;
- compute-budget failure;
- expiry without observation;
- duplicate identical submission;
- replacement transaction;
- multiple successful replacements;
- observer lag/disagreement;
- missing block height;
- unsupported durable nonce;
- Jito bundle states when the adapter exists.

### 23.6 Performance testing

- batch ingestion throughput;
- one-million-event trace and dashboard queries;
- projection backlog recovery;
- observer concurrency and rate limiting;
- report generation;
- SDK overhead;
- database storage per trace.

Reference hardware, versions, configuration, and dataset must be published with results.

## 24. Acceptance scenarios for R1

R1 is acceptable only when all of the following are demonstrated.

### AS-1 — First trace

A new user follows documentation, starts Docker Compose, instruments the example application, submits one devnet transaction, and sees a complete successful lifecycle in under 30 minutes.

### AS-2 — Telemetry fails open

The collector is stopped. The example application's transaction submission behavior remains successful or fails only for its original network reason. The SDK records/drops telemetry according to policy and does not throw by default.

### AS-3 — Expiration honesty

A controlled trace passes `lastValidBlockHeight` without observed inclusion. Landfall labels the state correctly, lists evidence, and states that the underlying propagation/scheduling cause is unknown.

### AS-4 — On-chain failure

A transaction is included with an execution error. The UI distinguishes landing from execution success and displays the normalized error evidence.

### AS-5 — Ambiguous client timeout

The application-side submission or confirmation wait times out, but the observer later sees execution. The timeline preserves both events and warns against blind replacement.

### AS-6 — Retry grouping

Three submissions of identical signed bytes produce one transaction trace with three attempts. A rebuilt transaction for the same business action produces a related second trace.

### AS-7 — Privacy

Secret fixtures and credential-bearing URLs do not appear in stored events, API responses, dashboard content, or exports.

### AS-8 — Comparison

Two application versions can be compared using completed observation windows. Sample counts, missing data, metric definitions, and changes are visible.

### AS-9 — Reproducible diagnosis

Replaying an event fixture with the same rule set produces the same lifecycle, diagnosis, certainty, and evidence references.

### AS-10 — Portable audit

The CLI exports an HTML/JSON report that opens without the running dashboard and states privacy mode, rule version, cohort, evidence, findings, and limitations.

## 25. Rollout plan

### Phase 0 — Evidence collection

- execute interviews from the validation strategy;
- manually analyze incidents;
- finalize the smallest useful event schema;
- choose the first JavaScript client adapter based on actual pilot code;
- confirm self-hosted versus hosted requirements.

### Phase 1 — R0 prototype

- implement domain types and event schema;
- implement CLI file ingestion;
- implement lifecycle projection;
- implement a small confirmed/unknown rule set;
- generate portable report;
- validate on five concierge datasets.

### Phase 2 — First paid pilot

- add only instrumentation required by the selected flow;
- deploy within customer's accepted boundary;
- establish baseline and data quality;
- deliver findings;
- measure accepted changes;
- collect security and usability objections.

### Phase 3 — Open-source R1

- generalize repeated pilot functionality;
- add collector, PostgreSQL, and dashboard;
- publish installation and privacy documentation;
- publish controlled fixtures and methodology;
- include an explicit limitations page;
- release under a selected license.

### Phase 4 — Recurring product decision

Use Gate D:

- build hosted/team functionality if recurring demand passes;
- maintain self-hosted paid support if that is the validated model;
- remain a productized consultancy if audits sell without software retention;
- stop commercial expansion if buyers do not pay.

## 26. Documentation requirements

P0 documentation must include:

- five-minute conceptual overview;
- architecture and data-flow diagram;
- supported and unsupported environments;
- Node.js integration guide;
- explicit event API guide;
- privacy modes and collected-field table;
- secret-handling guarantee and limitations;
- metric definitions;
- lifecycle-state definitions;
- diagnostic certainty definitions;
- rule catalog and versioning;
- deployment with Docker Compose;
- production-hardening checklist;
- troubleshooting and health check;
- controlled demo;
- report interpretation guide;
- limitations, including why a signature alone is insufficient;
- security contact and vulnerability-reporting process;
- contribution guide and code of conduct before public community growth.

## 27. Product and commercial analytics

The self-hosted product must not send usage analytics to Landfall by default.

For validation, usage evidence should be obtained through:

- customer-agreed pilot reports;
- optional explicitly enabled anonymous telemetry;
- scheduled review calls;
- recommendation dispositions;
- support interactions;
- customer-controlled exports.

If opt-in product telemetry is later added, it must:

- exclude signatures, addresses, endpoints, transaction payloads, and business IDs;
- document every field;
- be disabled by default in self-hosted mode;
- have a visible status and one-step disable control;
- never be required for core functionality.

## 28. Business model assumptions

The PRD supports three possible validated businesses:

### 28.1 Productized audit

- fixed price per transaction flow;
- software used to collect and analyze evidence;
- written report and remediation assistance;
- fastest path to initial revenue and case studies.

### 28.2 Self-hosted software and support

- open-source core;
- paid support, deployment, custom integrations, or enterprise features;
- attractive to trading and payments teams with sensitive telemetry.

### 28.3 Hosted monitoring

- monthly subscription based on event volume, retention, projects, and support;
- only justified after customers request continued monitoring and accept hosted data boundaries.

The product must not optimize for all three models simultaneously before validation.

## 29. Key risks and mitigations

| Risk | Impact | Mitigation / test |
|---|---|---|
| Existing providers solve enough of the problem | No willingness to pay | Interview multi-route teams; position around client-to-chain correlation and audits |
| Dropped transactions lack definitive evidence | Product disappoints users | Explicit certainty, data-quality guidance, no universal-cause promise |
| Instrumentation is too invasive | Low adoption | Explicit wrappers, non-blocking SDK, manual events, self-hosting |
| Telemetry reveals strategy or identity | Security rejection | Privacy modes, allowlist, local-first architecture, retention controls |
| Product adds latency or causes failures | Severe customer harm | Async fail-open design and overhead tests |
| Metrics count retries incorrectly | Misleading conclusions | Separate business action, trace, submission attempt, and replacement |
| Route comparison implies false attribution | Bad purchasing decisions | Show route receipts separately and label attribution limitations |
| Recommendations overfit one workload | Production regression | Advisory-only, evidence and limits, before/after measurement |
| Mainnet experiments become spammy or costly | Reputational/financial harm | Controlled low-volume budgeted methodology |
| Too much architecture before validation | Lost time | R0 file-based prototype and validation gates |
| Open-source users do not become buyers | No commercial result | Service-first pilots and explicit conversion measurement |
| Hosted SaaS creates premature compliance burden | Delayed launch | Self-hosted R1; hosted only after recurring demand |
| Solana client APIs evolve | Adapter maintenance | Neutral event schema and explicit compatibility matrix |
| Founder lacks credible Solana history | Sales resistance | Reproducible fixtures, honest limitations, public benchmarks, paid case studies |

## 30. Dependencies

- a validated customer segment and incident dataset;
- selected supported Solana JavaScript client;
- standard Solana JSON-RPC access for controlled testing;
- PostgreSQL and container runtime for R1;
- safe devnet/mainnet test funds with budget controls;
- customer agreement for any pilot telemetry;
- legal/license choice before public release;
- public vulnerability-reporting contact before production claims.

## 31. Open questions

These questions must be resolved through validation or implementation spikes, not preference alone.

### Market

1. Is transaction volume or economic value the best ICP threshold?
2. Which segment has the shortest paid-pilot sales cycle?
3. Is the primary buyer infrastructure engineering, trading, or the CTO?
4. Do customers pay for continuous monitoring or only for incident audits?
5. Is self-hosting mandatory in the strongest segment?

### Product

6. Which JavaScript client should be supported first?
7. Is a proxy/transport wrapper easier to adopt than explicit instrumentation?
8. What minimum event set produces actionable value in real incidents?
9. Does the first pilot require Jito bundle support?
10. Which business-action identifier can customers safely provide?
11. What retention period is acceptable?
12. Which recommendations repeat across at least three customers?

### Technical

13. Can a single PostgreSQL deployment meet pilot volume without pre-aggregation complexity?
14. How should status polling be scheduled to balance coverage, endpoint limits, and cost?
15. What is the correct rule for a late observation after locally declared expiration?
16. How should provider-specific errors map into a neutral taxonomy?
17. Can account-local fee context be computed locally and uploaded only as aggregate evidence?
18. How should durable-nonce transactions be detected without raw transaction storage?
19. What clock-skew checks are reliable across SDK and observer sources?
20. Which fields are safe and necessary for debugging versioned transactions and address lookup tables?

### Commercial and legal

21. What warranty and financial-risk disclaimers are required?
22. What data-processing terms will payment and trading teams require?
23. Should initial pricing be per audit, per project, per event, or support-based?

## 32. Decisions already made

| Decision | Rationale |
|---|---|
| Vendor-neutral observability, not a new sender | Existing infrastructure already specializes in delivery; correlation remains the proposed gap |
| Service-first commercial wedge | Produces revenue and evidence before a complete SaaS |
| Self-hosted open-source R1 | Reduces security objections and demonstrates technical capability |
| Apache License 2.0 for the open-source repository | Permissive commercial adoption plus an explicit patent grant; paid audits, deployment, support, integrations, and hosted operation remain available ([decision](licensing.md)) |
| Rust lifecycle/collector core | Fits performance, reliability, and portfolio goals without forcing all client integrations into Rust |
| TypeScript first application SDK | Likely integration surface for Solana applications; final library chosen from pilot evidence |
| No raw transaction storage by default | Minimizes sensitive data and unnecessary liability |
| No automatic mutation or retries in P0 | Prevents product recommendations from directly risking funds |
| Deterministic rule engine | Makes diagnoses reviewable and reproducible |
| Confirmed/probable/unknown certainty | Prevents overclaiming where dropped transaction evidence is incomplete |
| PostgreSQL before specialized data infrastructure | Minimizes premature operational complexity |
| Hosted SaaS postponed | Recurring demand and acceptable data boundaries are unvalidated |

## 33. Definition of MVP completion

The MVP is complete when:

1. Gate A and Gate B of the validation strategy have passed for a named ICP.
2. At least one paid pilot has used the product on a real transaction flow.
3. All R1 acceptance scenarios pass.
4. Product documentation states metric definitions, privacy behavior, and limitations.
5. The system produces a useful audit report from real sanitized data.
6. No private-key, seed, or RPC credential appears in security test storage or exports.
7. A customer can distinguish submission, landing, execution, and confirmation states in the UI.
8. The customer accepts at least one finding or reports materially faster diagnosis.
9. Reproducible fixtures and tests cover every authoritative diagnostic category.
10. The team has made an explicit Gate D decision about recurring SaaS, self-hosted support, consultancy, or portfolio-only continuation.

Completion is not defined by the number of implemented screens or rules. It is defined by safe operation, semantic correctness, and demonstrated customer value.

## Appendix A — Example trace narrative

> Business action `payout` created at 12:00:00.000. A recent blockhash was acquired at confirmed commitment with last valid block height 281,000,100. Simulation succeeded at 120,400 compute units. The transaction requested 130,000 units and was signed after 8.2 seconds. Identical signed bytes were submitted twice through route `rpc-primary`; the first request timed out and the second returned the signature. Status observer `observer-a` saw no status through block height 281,000,101. No other observer was configured. Landfall classifies the trace as expired without observed inclusion. That terminal observation is confirmed under the configured observer policy. The underlying propagation or scheduling cause is unknown. Compute headroom was 7.4%, which is flagged as a probable risk but not as the confirmed cause. Recommended actions: add a second observer for diagnostic coverage, record route response timing, acquire the blockhash closer to signing if signing latency grows, and review compute headroom under representative state.

This narrative demonstrates the required separation between observable result, possible risk, and unknown cause.

## Appendix B — Glossary

**Blockhash:** A recent network hash included in ordinary Solana transactions to establish a limited validity window.

**Commitment:** The level at which a client asks the cluster to report observed state, commonly processed, confirmed, or finalized.

**Compute unit (CU):** Unit used to meter transaction execution work.

**Data quality:** Completeness and consistency of telemetry required for diagnosis.

**Execution error:** Error produced when an included transaction's instructions execute unsuccessfully.

**Jito bundle:** Ordered group of transactions submitted to Jito infrastructure with bundle-specific acceptance and status semantics.

**Landing rate:** Share of eligible signed transactions observed included within a completed observation window.

**Last valid block height:** Boundary returned with a recent blockhash and used to determine its validity window.

**Observer:** RPC or specialized source used by Landfall to query block height, signature status, and transaction metadata.

**Priority fee:** Additional fee used in transaction scheduling priority according to the relevant Solana transaction format and network behavior.

**RPC acceptance:** A submission endpoint's successful response to a request; not proof of inclusion.

**Submission route:** RPC or specialized sender used to submit signed transaction bytes.

**Transaction signature:** Public identifier derived from signing a specific transaction message; its existence in the application does not prove inclusion.

## Appendix C — P0 requirement summary

P0 delivers:

- local project/environment configuration;
- neutral route labels and credential redaction;
- Node.js transaction lifecycle instrumentation;
- append-only schema and Rust collector;
- polling-based on-chain status observation;
- block-height-aware lifecycle derivation;
- confirmed/probable/unknown deterministic diagnostics;
- blockhash, compute, observability, and retry recommendations;
- stable metrics with correct retry semantics;
- overview, trace detail, comparison, and report export;
- Docker Compose self-hosted deployment;
- privacy modes, retention, deletion, and security fixtures;
- controlled examples and reproducible golden incidents.

P0 explicitly does not deliver custody, signing, automatic resubmission, guaranteed cause attribution, generalized chain indexing, enterprise multi-tenancy, or automated strategy changes.
