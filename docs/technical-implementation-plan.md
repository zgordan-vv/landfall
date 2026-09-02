# Landfall Technical Implementation Plan

Status: Draft; no implementation started  
Version: 0.1  
Date: 2026-08-29  
Working product name: **Landfall**  
Related documents: [Product Requirements Document](./product-requirements-document.md), [System Design](./system-design.md), [P0 Support Matrix](./support-matrix.md), [Idea Validation Strategy](./idea-validation-strategy.md)

## 1. Purpose

This document converts the Landfall PRD and system design into a dependency-ordered implementation plan. It specifies the proposed technology stack, repository layout, engineering standards, milestones, concrete outputs, tests, acceptance gates, and portfolio deliverables.

The plan is intentionally instructional. During implementation, each phase will be preceded by an explanation of its architecture and followed by a walkthrough of every created or changed file, important type, function, query, test, and trade-off.

This document does not authorize or perform implementation. Coding begins only after an explicit user instruction.

## 2. Implementation objectives

The implementation must achieve four outcomes simultaneously:

1. **Correct product semantics:** submission, landing, execution, confirmation, expiration, retry, replacement, and business outcome remain distinct.
2. **Production-oriented engineering:** idempotency, security, backpressure, failure recovery, migrations, metrics, and tests are first-class.
3. **Understandable architecture:** one developer can explain the complete data flow and justify every major decision in an Upwork interview.
4. **Visible portfolio value:** a reviewer can run a demo, inspect traces, trigger controlled failures, view diagnoses, and read reproducible reports.

## 3. Technology stack

Versions below began as the planning baseline on 2026-08-29. The exact P0
runtime/client/database lane was verified and frozen on 2026-09-01 in the
[support matrix](support-matrix.md); remaining implementation dependencies are
pinned in lockfiles and container digests when their phase begins.

### 3.1 Runtime and database baseline

| Area | Choice | Baseline | Why |
|---|---|---:|---|
| Backend language | Rust | 1.98.0, Edition 2024 | Memory safety, strong domain modeling, concurrency, performance, portfolio relevance |
| JavaScript runtime | Node.js | 24 LTS; 24.20.0 P0 pin | Supported production LTS; exact lane is owned by the support matrix |
| Type system/compiler | TypeScript | 7.0 | Current stable native compiler and strong SDK/frontend contracts |
| Database | PostgreSQL | 18.6 P0 pin | Durable relational/event storage, JSONB, transactions, partitioning, job leases |
| Container runtime | Docker + Compose | Compose v2 | Reproducible two-container self-hosted deployment |
| Shell task runner | `just` | pinned stable | One discoverable command surface across Rust, Node, database, and Docker |
| JS package manager | pnpm | current stable, pinned | Workspaces, deterministic lockfile, efficient monorepo installs |

Official version references:

- Rust 1.98 is the current stable release: [Rust release announcement](https://blog.rust-lang.org/2026/08/20/Rust-1.98.0/).
- Node 24 is LTS while Node 26 is Current: [Node release schedule](https://nodejs.org/en/about/previous-releases).
- PostgreSQL 18 is supported through 2030: [PostgreSQL versioning policy](https://www.postgresql.org/support/versioning/).
- TypeScript 7 is stable: [TypeScript 7 announcement](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/).

### 3.2 Rust backend stack

| Concern | Library | Baseline | Role |
|---|---|---:|---|
| Async runtime | Tokio | 1.52 | Tasks, timers, cancellation, networking |
| HTTP server | Axum | 0.8 | REST API, extractors, response/error model |
| Middleware | Tower / tower-http | compatible pinned versions | Timeouts, request IDs, body limits, tracing, CORS in development |
| Serialization | Serde / serde_json | 1.x | Event, API, RPC, and configuration encoding |
| Database | SQLx | 0.9 | Async PostgreSQL, migrations, compile-time checked queries |
| HTTP/RPC client | Reqwest | 0.13 | Solana JSON-RPC with pooled connections and Rustls |
| OpenAPI | Utoipa | 5.5 | OpenAPI 3.1 generation from API types/routes |
| CLI | Clap | 4.6 | Commands, help, completions, typed options |
| IDs | `uuid` | 1.x with v7 feature | Event, trace, job, and resource identity |
| Time | `time` | 0.3 | UTC timestamps and duration formatting |
| Errors | `thiserror` | 2.x | Typed library/domain errors |
| App boundaries | `anyhow` | 1.x | CLI/startup context only, not public domain errors |
| Logging | `tracing`, `tracing-subscriber` | 0.1 / compatible | Structured spans and operational logs |
| Hashing | `sha2`, `hmac`, `subtle` | pinned stable | Digests, keyed privacy mode, constant-time comparison |
| Secrets | `secrecy`, `zeroize` where applicable | pinned stable | Reduce accidental display/copy of token material |
| Config | `config` or explicit layered loader | decided by ADR | Env/file configuration with strict validation |
| Templates | Askama | 0.16 | Compile-time checked, self-contained HTML report rendering; selected by the report spike |
| Schema validation | `jsonschema` | pinned stable | Contract validation for fixtures and ingestion defense |
| Property tests | Proptest | pinned stable | Event-order, idempotency, and state invariants |
| Snapshots | Insta | pinned stable | Reports, OpenAPI, diagnoses, and error contracts |
| HTTP mocks | Wiremock | pinned stable | Solana RPC behavior and retry tests |

Current verified framework baselines include Axum 0.8.9, Tokio 1.52.3, SQLx 0.9.0, and Reqwest 0.13.4. [Axum](https://docs.rs/axum/latest/axum/), [Tokio](https://docs.rs/tokio/latest/tokio/), [SQLx](https://docs.rs/sqlx/latest/sqlx/), [Reqwest](https://docs.rs/reqwest/latest/reqwest/).

Rust features will be kept minimal rather than using every crate's `full` feature. The production HTTP client will use Rustls rather than platform-native TLS for consistent container builds unless a measured compatibility issue requires otherwise.

### 3.3 TypeScript SDK stack

| Concern | Choice | Role |
|---|---|---|
| Language | TypeScript 7, strict mode | Public SDK contract and compile-time safety |
| Runtime target | Node 24 LTS, ESM first | Server-side instrumentation |
| Package build | `tsc` plus a small library bundler selected at bootstrap | Declarations and ESM/CJS distribution if required |
| Validation | Generated types plus focused runtime guards | Prevent malformed event construction |
| Tests | Vitest 4.1 | Unit, fake-timer, transport, and adapter tests |
| HTTP mocking | MSW or Undici mock agent | Collector transport tests |
| Solana adapter | `@solana/kit` 8.2.0 first | Exact initial compatibility lane from the P0 support matrix |
| Compatibility | Neutral manual API; then a web3.js v3 spike and legacy v1 only if evidence requires | Existing-client adoption without a false broad support claim |

Solana currently recommends `@solana/kit`. The exact P0 lane and unsupported
fallback behavior are frozen in the [support matrix](support-matrix.md) and
[ADR-006](adr/006-solana-kit-first-adapter-and-compatibility-roadmap.md).

The neutral manual event API is implemented before any Solana-specific adapter. That makes the SDK useful for custom clients and prevents the backend protocol from depending on one library's object model.

### 3.4 Dashboard stack

| Concern | Choice | Baseline/role |
|---|---|---|
| UI | React | 19.2 stable |
| Build | Vite | 8.1 line |
| Language | TypeScript | 7.0 strict |
| Routing | React Router | pinned stable |
| Server-state queries | TanStack Query | pinned stable; caching, retries, polling |
| Tables | TanStack Table | accessible, headless trace tables |
| Styling | Tailwind CSS 4 plus CSS variables | Fast consistent dashboard system |
| Accessible primitives | Radix UI where native HTML is insufficient | Dialogs, popovers, menus |
| Icons | Lucide React | Small, consistent icons |
| Charts | Recharts or ECharts selected by a spike | Time series, distributions, comparison |
| Unit/component tests | Vitest 4.1 + Testing Library | Behavior and accessibility |
| API mocking | MSW | Deterministic frontend scenarios |
| End-to-end tests | Playwright | Real browser against the composed stack |

React 19.2 is the current documented major/minor line, and Vite 8 is stable. [React versions](https://react.dev/versions), [Vite 8 announcement](https://vite.dev/blog/announcing-vite8), [Vitest 4.1](https://vitest.dev/blog/vitest-4-1).

No server-side React framework is needed. The dashboard is a private self-hosted SPA backed by the Rust REST API; adding Next.js would duplicate server responsibilities and complicate deployment.

### 3.5 Contracts and code generation

The project has two public contract families:

1. **Event protocol:** versioned JSON Schema files under `schemas/events/v1/` are canonical. Rust Serde types and generated/verified TypeScript types must pass the same golden fixtures.
2. **REST API:** Rust request/response types and Utoipa annotations produce OpenAPI 3.1. The checked-in OpenAPI snapshot generates the dashboard/CLI TypeScript API types.

CI fails if:

- generated schemas/OpenAPI differ from committed artifacts;
- Rust or TypeScript rejects a golden valid fixture;
- either language accepts a golden invalid fixture expected to fail;
- an API change occurs without a version/changelog decision.

This contract-first discipline is intentionally visible in the portfolio: it demonstrates safe polyglot integration rather than manually duplicated interfaces.

### 3.6 Development and quality tooling

#### Rust

- `rustfmt` for formatting;
- Clippy with warnings denied in CI;
- `cargo nextest` for test execution;
- `cargo llvm-cov` for coverage reporting;
- `cargo deny` for licenses, advisories, duplicate/dependency policy;
- `cargo audit` as an advisory check;
- SQLx offline metadata checked into the repository when compile-time queries are used.

#### TypeScript

- pnpm workspace;
- ESLint flat configuration with `typescript-eslint` and React rules;
- Prettier or the selected formatter, with one formatter only;
- Vitest;
- Playwright;
- dependency and license checks;
- package export tests against Node ESM and, if shipped, CJS.

#### Repository

- GitHub Actions;
- Docker Buildx;
- Trivy container/filesystem scan;
- Gitleaks secret scan;
- Renovate or Dependabot after initial release;
- Conventional Commits are optional, but PR titles and changelog entries must be descriptive;
- Architecture Decision Records for irreversible/high-cost choices.

### 3.7 Explicitly rejected P0 technologies

| Technology | Why not in P0 |
|---|---|
| Kafka/NATS | PostgreSQL can handle the target event rate and provide atomic event-plus-job persistence |
| Redis | No validated cache or queue requirement; in-process cache plus PostgreSQL is sufficient |
| ClickHouse | Thirty-day single-team analytics can be served from projections/PostgreSQL |
| Kubernetes | Docker Compose is easier to run, explain, and support for the first customer |
| GraphQL | REST maps naturally to trace/search/report resources and is easier to document/curl |
| gRPC ingestion | Browser/debugging friction and unnecessary protocol complexity at the target volume |
| Next.js | No SSR/public SEO need; Rust already owns the server |
| ORM | SQLx preserves explicit SQL, query plans, and PostgreSQL features |
| LLM diagnosis | Authoritative rules must be deterministic and evidence-linked |
| Raw Solana transaction proxy | Landfall must not become the transaction critical path |

## 4. Planned repository structure

The refined implementation layout is:

```text
/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── package.json
├── pnpm-lock.yaml
├── pnpm-workspace.yaml
├── tsconfig.base.json
├── justfile
├── docker-compose.yml
├── .env.example
├── .editorconfig
├── .github/
│   └── workflows/
├── apps/
│   └── dashboard/
├── crates/
│   ├── landfall-protocol/      # Wire types, IDs, schema/version vocabulary
│   ├── landfall-core/          # Pure reducer, rules, metrics, domain projections
│   ├── landfall-storage/       # SQLx repositories, migrations, PostgreSQL job queue
│   ├── landfall-observer/      # Solana JSON-RPC client and observation scheduler
│   ├── landfall-report/        # Report model and HTML/JSON rendering
│   ├── landfall-server/        # Axum API, workers, configuration, static UI serving
│   └── landfall-cli/           # Init, doctor, ingest, trace, report, demo commands
├── packages/
│   ├── protocol-ts/            # Generated/verified event protocol types
│   ├── api-client/             # Generated REST API types/client
│   └── sdk-ts/                 # Public Node instrumentation SDK and adapters
├── schemas/
│   ├── events/v1/
│   └── openapi/
├── migrations/
├── examples/
│   ├── kit-node/
│   ├── manual-events/
│   └── fixtures/
├── deployments/
│   └── docker-compose/
├── benchmarks/
├── scripts/
└── docs/
    ├── adr/
    ├── runbooks/
    └── existing product/design documents
```

### 4.1 Crate dependency direction

```text
landfall-protocol
      ↑
landfall-core
      ↑
landfall-storage     landfall-observer     landfall-report
          \              |                /
                    landfall-server
                           ↑
                     landfall-cli
```

Rules:

- `protocol` depends on no application/database/web crate;
- `core` may depend on `protocol`, never Axum/SQLx/Reqwest;
- `storage`, `observer`, and `report` depend on protocol/core interfaces;
- `server` composes infrastructure;
- `cli` calls public service interfaces and must not bypass domain invariants;
- cyclic crate dependencies are prohibited.

## 5. Engineering workflow for every implementation step

Each phase follows the same teaching and delivery loop.

### Before coding

1. State the user/system problem being solved.
2. Draw or explain the data flow.
3. Identify new files/modules and their responsibilities.
4. Explain the important types and invariants.
5. Compare realistic alternatives.
6. State tests and exit criteria before implementation.
7. Identify security, performance, and Solana-specific concerns.

### During coding

1. Make a small coherent change.
2. Keep domain code separate from infrastructure.
3. Add tests with the implementation, not afterward.
4. Run the smallest relevant verification frequently.
5. Record an ADR when a decision changes future architecture.
6. Avoid unexplained generated code or copied boilerplate.

### After coding

1. Walk through every created/modified file.
2. Explain important code block by block.
3. Trace one real request/event through the component.
4. Show success, expected failure, and dependency failure behavior.
5. Run tests and explain what each class of test proves.
6. Update architecture/API/schema documentation.
7. Explain how to present the work to an Upwork client.

### Definition of done for a phase

- functional exit criteria pass;
- relevant tests pass locally and in CI;
- formatting/lint/type checks pass;
- security-sensitive behavior has a negative test;
- docs and examples match the code;
- no unresolved critical TODO is hidden in source;
- the feature can be demonstrated or inspected independently.

## 6. Milestone overview

| Phase | Milestone | Primary output | Depends on |
|---:|---|---|---|
| 0 | Design lock and ADRs | Frozen choices and threat model | Existing docs |
| 1 | Monorepo foundation | Reproducible workspace and CI | 0 |
| 2 | Event protocol | Schemas, types, fixtures | 1 |
| 3 | Domain core | Deterministic lifecycle/rules | 2 |
| 4 | R0 offline vertical slice | NDJSON → diagnosis → report CLI | 3 |
| 5 | PostgreSQL foundation | Migrations, repositories, jobs | 2–3 |
| 6 | Collector API | Durable idempotent event ingestion | 5 |
| 7 | Projection pipeline | Events → read model/diagnoses | 3, 5, 6 |
| 8 | TypeScript SDK | Safe async event producer | 2, 6 |
| 9 | Solana Kit adapter | Instrumented real client flow | 8 |
| 10 | Observer | Signature/blockheight/enrichment | 5, 7 |
| 11 | Query and metrics API | Trace/search/comparison endpoints | 7, 10 |
| 12 | Dashboard | Visual investigation workflow | 11 |
| 13 | Reports, CLI, administration | Audits and operator workflows | 11–12 |
| 14 | Security and resilience | Hardened failure/privacy behavior | All previous |
| 15 | Capacity and performance | Verified design targets | 14 |
| 16 | Deployment and portfolio release | Runnable demo, docs, release artifacts | 15 |

Phases create vertical evidence. We do not build the entire backend before the first usable result: Phase 4 already produces a portable offline diagnostic report.

## 7. Phase 0 — Design lock and engineering decisions

**Status:** Completed on 2026-09-01.

### Goal

Resolve choices that would otherwise leak ambiguity into contracts and persistence.

### Tasks

1. Create ADR template and index.
2. Record ADR-001: modular monolith and two-container topology.
3. Record ADR-002: immutable event inputs plus relational projections.
4. Record ADR-003: identifier model—business action, trace, attempt, event, alias.
5. Record ADR-004: privacy modes and signed-byte fingerprint strategy.
6. Record ADR-005: JSON Schema event contract and code-generation direction.
7. Record ADR-006: Solana Kit first adapter and compatibility roadmap.
8. Record ADR-007: PostgreSQL job queue instead of external broker.
9. Record ADR-008: code-first OpenAPI via Utoipa.
10. Select report renderer and artifact storage through a tiny documented spike. Completed: [Askama and PostgreSQL `BYTEA`](spikes/report-renderer-and-artifact-storage.md).
11. Create an implementation threat model using the assets and threats in the PRD/system design. Completed: [P0 implementation threat model](threat-model.md).
12. Freeze P0 support matrix: Node, PostgreSQL, Solana clusters, transaction versions, and privacy modes. Completed: [P0 support matrix](support-matrix.md).

### Key decisions recommended

- fingerprint: `lf-hmac-sha256-v1` with an environment key in every P0 mode; plain SHA-256 is unsupported;
- first client adapter: exact `@solana/kit` 8.2.0 on Node.js 24 LTS;
- P0 transaction versions: legacy and v0; v1 is explicitly detected and reported unsupported until parser tests exist;
- reports: PostgreSQL `BYTEA` capped at 10 MiB for P0;
- queue notification: polling is authoritative; PostgreSQL `LISTEN/NOTIFY` may be a latency hint only.

### Deliverables

- `docs/adr/000-template.md` and ADR files;
- `docs/threat-model.md`;
- `docs/support-matrix.md`;
- resolved update to the system-design open decisions.

### Exit gate

No unresolved choice affects event identity, privacy, schema source of truth, or supported transaction format.

### Portfolio explanation

Demonstrates that architecture is driven by constraints and recorded decisions, not framework fashion.

## 8. Phase 1 — Monorepo and delivery foundation

### Goal

Create a reproducible empty workspace where one command verifies all languages and where CI matches local development.

### Tasks

1. Initialize Cargo workspace and seven crates with dependency direction enforced. Completed: [Rust crate boundaries](../crates/README.md).
2. Pin Rust 1.98 and required components in `rust-toolchain.toml`. Completed: [Rust toolchain pin](../rust-toolchain.toml).
3. Configure workspace lints, Rust Edition 2024, release profile, and minimal features.
4. Initialize pnpm workspace, Node 24 pin, TypeScript base config, and package boundaries.
5. Create dashboard and package skeletons without product logic.
6. Add `.editorconfig`, ignore files, license placeholder/decision, security policy, and contribution basics.
7. Create `just` commands: `bootstrap`, `fmt`, `lint`, `typecheck`, `test`, `test-integration`, `build`, `check`, `dev`, `db-up`, `db-reset`.
8. Add Docker Compose with PostgreSQL health check; server may be a placeholder health binary only when implementation begins.
9. Create GitHub Actions jobs:
   - Rust format/lint/test;
   - TypeScript lint/typecheck/test/build;
   - PostgreSQL integration tests;
   - schema/OpenAPI drift;
   - dependency/license/secret scan;
   - container build.
10. Enable caching without making CI depend on cached artifacts.
11. Add a repository architecture README with links to PRD, design, and plan.

### Expected files

- root workspace/configuration files;
- crate/package manifests;
- `.github/workflows/ci.yml`;
- minimal Docker Compose;
- `README.md` and `CONTRIBUTING.md`.

### Tests and checks

- fresh clone bootstrap;
- `just check` on clean machine/container;
- all skeleton crates/packages build;
- PostgreSQL health check succeeds;
- dependency direction test or documented review rule.

### Exit gate

A new developer can clone the repository and run one documented command to reproduce CI successfully.

### Portfolio explanation

Shows polyglot monorepo organization, reproducibility, CI, and professional project hygiene before feature code.

## 9. Phase 2 — Event protocol and golden fixtures

### Goal

Define the immutable language that connects customer applications, Rust services, reports, and tests.

### Tasks

1. Enumerate P0 event types and common envelope.
2. Define JSON Schema for:
   - trace created;
   - blockhash acquired;
   - simulation started/completed;
   - signing started/completed;
   - submission started/completed;
   - retry scheduled;
   - confirmation wait started/completed;
   - status observed;
   - execution enriched;
   - business outcome observed.
3. Define decimal-string rules for values beyond JavaScript safe integer range.
4. Define enums for privacy mode, commitment, transport result, RPC result, and normalized errors.
5. Implement Rust wire types in `landfall-protocol`.
6. Generate or verify `protocol-ts` types.
7. Create valid and invalid fixture corpus.
8. Add schema version compatibility rules.
9. Add redaction fixtures with keys, bearer tokens, RPC URLs, cookies, and oversized metadata.
10. Document examples and field-level privacy classification.

### Golden incident fixture set

- successful transaction;
- simulation error;
- submission rejection;
- transport timeout followed by later success;
- expiry without observed inclusion;
- compute-budget execution error;
- repeated identical submission;
- replacement transaction for one business action;
- two successful replacements;
- observer disagreement;
- missing block height;
- unsupported durable nonce or transaction version.

### Tests

- Rust and TypeScript accept all valid fixtures;
- both reject invalid fixtures with stable categories;
- JSON round trips do not change canonical values;
- large integer values retain precision;
- unknown fields follow schema version policy;
- secret fixtures cannot enter allowed event attributes.

### Exit gate

The protocol is sufficient to express every R1 acceptance scenario without storing raw signed transactions.

### Portfolio explanation

Shows schema design, cross-language contracts, backward compatibility, precision handling, and privacy classification.

## 10. Phase 3 — Deterministic domain core

### Goal

Implement Landfall's most important intellectual property independently of network, database, and UI code.

### Tasks

1. Define domain entities and newtypes:
   - IDs;
   - business action;
   - transaction trace;
   - submission attempt;
   - observation;
   - simulation;
   - execution metadata;
   - diagnostic;
   - recommendation.
2. Define orthogonal state dimensions:
   - lifecycle stage;
   - landing state;
   - execution result;
   - application outcome;
   - observation completeness.
3. Implement canonical event ordering with clock-quality warnings.
4. Implement the pure trace reducer.
5. Implement retry versus replacement grouping rules.
6. Implement data-quality grading.
7. Implement initial confirmed rules:
   - simulation error;
   - RPC rejection;
   - on-chain execution error;
   - compute-budget failure;
   - validity window passed without observed inclusion;
   - client timeout followed by network success.
8. Implement initial probable rules:
   - excessive signing delay;
   - low compute headroom;
   - route degradation signal;
   - unsafe/redundant retry;
   - fee likely uncompetitive only when required evidence exists.
9. Implement unknown/missing-evidence generation.
10. Implement advisory recommendations and evidence references.
11. Implement metric definitions as pure functions over projections.
12. Version reducer, rule set, and metric definitions.

### Internal module plan

```text
crates/landfall-core/src/
├── domain/
├── ordering/
├── reducer/
├── data_quality/
├── diagnostics/
│   └── rules/
├── recommendations/
├── metrics/
└── report_model/
```

### Tests

- one fixture test per authoritative rule;
- near-miss test ensuring probable/unknown is not promoted;
- event permutation property tests;
- duplicate event property tests;
- impossible-state tests;
- retry/replacement tests;
- metric denominator tests;
- snapshot of diagnosis evidence and wording keys;
- deterministic replay test with the same rule version.

### Exit gate

All golden fixtures produce the PRD-required state, certainty, evidence, recommendation, and metric behavior without database or HTTP access.

### Portfolio explanation

This phase is the strongest interview material: type-driven design, deterministic event reduction, causal honesty, property testing, and financial-safety boundaries.

## 11. Phase 4 — R0 offline vertical slice

### Goal

Produce the first end-to-end usable artifact before building continuous infrastructure.

### Tasks

1. Implement `landfall ingest <ndjson>`.
2. Stream and validate NDJSON rather than load the entire file.
3. Group events into canonical traces and aliases in memory.
4. Run reducer, data quality, diagnoses, recommendations, and metrics.
5. Build a stable report-domain model.
6. Render structured JSON.
7. Render self-contained HTML with evidence timeline and limitations.
8. Add `--privacy-profile` export redaction.
9. Add example input and expected report snapshots.
10. Document how to inspect each golden incident.

### Tests

- CLI exit codes for valid/invalid input;
- bounded memory fixture;
- deterministic JSON report snapshot;
- HTML semantic/content snapshot;
- report secret scan;
- report opens without server assets;
- 100,000-trace synthetic report benchmark baseline.

### Exit gate

One command converts a fixture dataset into an understandable audit report containing correct certainty and evidence.

### Portfolio demo

“Here is raw instrumentation; here is the deterministic analysis; here is a portable incident report.” This is already useful even if later platform work stops.

## 12. Phase 5 — PostgreSQL persistence and job infrastructure

### Goal

Create the durable foundation for continuous ingestion and asynchronous processing.

### Tasks

1. Add SQLx and PostgreSQL connection configuration.
2. Create migrations for schemas:
   - `control`;
   - `telemetry`;
   - `work`;
   - `reporting`.
3. Implement projects, environments, routes, and API-token tables.
4. Implement event dedup registry and daily raw-event partition parent.
5. Implement business actions, traces, aliases, typed child projections.
6. Implement diagnostics, evidence, recommendations, and disposition history.
7. Implement jobs with lease fields and active dedupe keys.
8. Implement report metadata/artifact tables.
9. Build repository traits at service boundaries and SQLx implementations.
10. Implement migration runner and version health check.
11. Implement test database lifecycle and seed helpers.
12. Add partition creation/retention primitives.
13. Inspect query plans for trace lookup and bounded time search.

### Database engineering rules

- migrations are append-only after release;
- destructive changes require expand/migrate/contract approach;
- no arbitrary GIN index on event JSON;
- all list queries include environment and bounded time;
- transactions are explicit at service boundaries;
- test data uses realistic cardinality and timestamp distribution.

### Job queue algorithm

1. Insert job with type/dedupe key.
2. Worker selects ready rows using `FOR UPDATE SKIP LOCKED`.
3. Worker assigns `locked_by` and `locked_until` in a short transaction.
4. External work occurs outside the claim transaction.
5. Completion, retry, or dead-letter update is explicit.
6. Expired leases become claimable.

### Tests

- all migrations from empty database;
- migration checksum/drift;
- event dedup under concurrent inserts;
- job claim concurrency;
- worker crash/lease expiry;
- repository transaction rollback;
- trace/signature unique identity;
- alias resolution;
- partition creation and deletion;
- query-plan snapshots for critical queries.

### Exit gate

PostgreSQL can durably store/replay events, project identities, and recover background jobs after a forced worker termination.

### Portfolio explanation

Demonstrates explicit SQL, transactional outbox/job semantics, concurrency control, partitioning, and operational migrations.

## 13. Phase 6 — Collector ingestion API

### Goal

Accept authenticated, validated, idempotent event batches and durably enqueue projection work.

### Tasks

1. Build Axum application state and router composition.
2. Add request IDs and structured tracing spans.
3. Add liveness/readiness endpoints.
4. Implement bearer-token hashing, lookup, scope, expiration, and revocation.
5. Implement compressed and decompressed body limits.
6. Implement batch schema/semantic validation.
7. Repeat privacy enforcement server-side.
8. Implement atomic transaction:
   - dedup claim;
   - raw-event insert;
   - minimal identity upsert;
   - projection job enqueue.
9. Implement `202`, duplicate `200`, and structured error responses.
10. Implement token/request rate and concurrency limits.
11. Generate OpenAPI route and schema snapshot.
12. Add disposable health-check event endpoint.

### Tests

- API contract tests for every status code;
- replay same batch/event IDs;
- partial duplicate batch;
- invalid event causes atomic batch rejection;
- wrong environment scope;
- token expiration/revocation;
- gzip bomb/oversized body;
- credential-bearing field rejection;
- database unavailable returns 503 without false acceptance;
- commit-before-202 restart test;
- OpenAPI snapshot.

### Exit gate

The API's `202 Accepted` has a precise demonstrated meaning: every non-duplicate event is committed and corresponding projection work is durable.

### Portfolio explanation

Shows HTTP middleware, authentication, idempotency, backpressure, database transactions, OpenAPI, and secure error design.

## 14. Phase 7 — Projection and diagnosis pipeline

### Goal

Turn newly ingested events into queryable trace state and versioned diagnoses.

### Tasks

1. Implement server worker supervisor and cancellation.
2. Implement `project_trace` job worker.
3. Resolve trace aliases by signature/digest.
4. Load canonical raw events.
5. Run deterministic reducer.
6. Replace/upsert typed child projections in one transaction.
7. Preserve/supersede diagnosis history.
8. Persist recommendation and evidence links.
9. Increment projection version/watermark.
10. Enqueue observation only when eligible.
11. Add projection-lag and dead-job metrics.
12. Add reproject command for one trace, time range, or rule version.

### Tests

- out-of-order event arrival;
- concurrent jobs for one trace;
- new event after terminal projection;
- rule-set upgrade preserves old diagnosis;
- rollback leaves old projection intact;
- poison event/job reaches dead state with visible error code;
- full raw replay reproduces projection;
- projection lag under synthetic burst.

### Exit gate

An ingested golden fixture becomes the same projection/report result as the offline R0 path.

### Portfolio explanation

Shows event-driven processing, eventual consistency, deterministic replay, job recovery, and historical rule versioning.

## 15. Phase 8 — TypeScript instrumentation SDK

### Goal

Provide a safe, pleasant producer library without putting Landfall on the customer transaction critical path.

### Tasks

1. Design minimal public API and error policy.
2. Implement validated immutable configuration.
3. Implement `BusinessActionContext` and `TraceContext`.
4. Implement typed manual lifecycle event builders.
5. Implement UUIDv7 generation/canonical format.
6. Implement signed-byte fingerprinting according to ADR.
7. Implement endpoint redaction and metadata allowlist.
8. Implement bounded in-memory buffer.
9. Implement batch assembly and stable retries.
10. Implement HTTP/gzip transport and backoff with jitter.
11. Implement drop/transport health counters and customer callback.
12. Implement bounded shutdown flush.
13. Build ESM package and declarations; add CJS only if support decision requires it.
14. Create manual-events example.

### Public API qualities

- explicit rather than global monkey patching;
- no private key parameter anywhere;
- normal telemetry errors do not throw through customer submission;
- advanced methods remain behind a namespaced API;
- generated protocol types are not all exposed as public user complexity;
- tree-shakeable or at least small enough for server use, with measured package size.

### Tests

- fake-timer batching;
- flush size/time boundaries;
- transport retry with stable IDs;
- overflow behavior;
- collector outage does not reject wrapped customer promise;
- endpoint/secret redaction;
- large integer serialization;
- Node process shutdown;
- ESM import/package exports;
- compatibility with Node 24.

### Exit gate

The manual example sends a complete golden trace to the real collector, while a collector outage leaves the simulated customer transaction behavior unchanged.

### Portfolio explanation

Shows SDK design, async buffering, failure isolation, privacy, packaging, and cross-language contracts.

## 16. Phase 9 — Solana Kit adapter and example application

### Goal

Instrument a real modern Solana transaction lifecycle with minimal application changes.

### Tasks

1. Implement and verify the frozen `@solana/kit` 8.2.0 lane; add any required plugin tuple to the support matrix before release.
2. Define adapter boundary independent of Kit internals.
3. Capture blockhash and `lastValidBlockHeight`.
4. Capture simulation result and compute units when available.
5. Measure signing delay without accessing signer secrets.
6. Fingerprint signed bytes in memory.
7. Wrap submission attempts with route and configuration.
8. Capture application confirmation-wait result.
9. Build a controlled SOL-transfer or harmless-instruction example for local validator/devnet.
10. Demonstrate success, simulation failure, client timeout/later success, and expiration fixtures.
11. Document the integration and every captured field.
12. After the Kit lane is stable, run a time-boxed `@solana/web3.js` v3 compatibility spike; defer production support and a legacy v1 adapter until evidence justifies them.

### Security constraints

- adapter never accepts or exports raw keypair material;
- logs never contain signed bytes;
- full transaction object is not serialized into custom metadata;
- RPC endpoint credentials are mapped to route IDs;
- demo funds and networks are explicit.

### Tests

- adapter unit tests with mocked Kit client;
- local validator integration;
- devnet opt-in test;
- blockhash/simulation/submission field correctness;
- preflight skipped behavior;
- route retry and replacement grouping;
- unsupported transaction-version behavior.

### Exit gate

A documented Kit application produces a complete trace in under 30 minutes from a fresh setup.

### Portfolio explanation

Shows current Solana Kit knowledge while the neutral protocol proves the platform is not coupled to one SDK.

## 17. Phase 10 — Solana observer and enrichment

### Goal

Observe network outcomes independently from the customer's submission path.

### Tasks

1. Implement provider-neutral JSON-RPC client and normalized errors.
2. Implement route-specific pooled Reqwest client with Rustls.
3. Implement `getBlockHeight` shared cache/worker.
4. Implement durable observation schedule and in-memory priority queue.
5. Batch `getSignatureStatuses` by route/options, default 100, maximum 256.
6. Implement adaptive polling and per-route rate limits.
7. Persist first null, state changes, periodic checkpoints, RPC error transitions, and terminal evidence.
8. Implement expiry evaluation from current block height and `lastValidBlockHeight`.
9. Enqueue and implement `getTransaction` enrichment.
10. Normalize execution error, fee, compute units, slot, block time, and transaction version.
11. Implement observer route health and data-quality gaps.
12. Detect durable-nonce/unsupported validity cases without false expiration.
13. Support clean cancellation and restart recovery.

### Tests

- Wiremock sequences for null → processed → confirmed → finalized;
- RPC timeout/rate limit/server error;
- batch response index alignment;
- block-height cache and stale source;
- exact validity boundary;
- late observation after prior local conclusion;
- `getTransaction` null before available;
- legacy and v0 parsing;
- unsupported v1 behavior;
- rate limiter fairness between status and enrichment;
- worker restart from durable due jobs.

### Exit gate

The controlled application trace progresses from submitted to observed outcome without application-supplied final state, and an unobserved trace expires with honest unknown cause language.

### Portfolio explanation

Shows Solana RPC semantics, batching, rate limiting, adaptive polling, eventual consistency, and careful treatment of absence.

## 18. Phase 11 — Query, metrics, comparison, and control APIs

### Goal

Expose the platform's read model through stable documented APIs.

### Tasks

1. Implement cursor-pagination utility.
2. Implement trace list filters and bounded-time validation.
3. Implement trace detail with attempts, observations, evidence, recommendations, and watermark.
4. Implement signature lookup with privacy/log safeguards.
5. Implement business-action detail and multiple-success warning.
6. Implement metric summary with numerator/denominator/exclusions.
7. Implement data-quality summary.
8. Implement descriptive cohort comparison.
9. Implement recommendation disposition and history.
10. Implement project/environment/route read configuration endpoints.
11. Implement authenticated detailed system status.
12. Add ETag/projection version behavior.
13. Generate OpenAPI and TypeScript API client.

### Tests

- every filter and invalid combination;
- stable cursor under equal timestamps;
- strict privacy signature lookup disabled;
- terminal denominator correctness;
- retries do not inflate transaction counts;
- unknown fee components excluded rather than zero;
- route attribution limitation;
- small-sample warning;
- ETag changes only with projection version;
- query statement timeouts and maximum ranges;
- generated API client compilation.

### Exit gate

All dashboard-required data can be obtained through documented API calls, with no direct database access from frontend code.

### Portfolio explanation

Shows API design, cursor pagination, analytical correctness, privacy-aware search, code generation, and query performance.

## 19. Phase 12 — Dashboard

### Goal

Make lifecycle evidence and uncertainty understandable within seconds.

### Step 12.1 — Frontend foundation

- React/Vite/TypeScript setup;
- generated API client;
- application shell and routes;
- design tokens, typography, status semantics;
- accessible components;
- error boundary and loading/empty states;
- MSW fixture environment.

### Step 12.2 — Onboarding and system health

- project/environment status;
- first-trace checklist;
- collector/observer/schema/privacy display;
- CLI command hints;
- data-quality warnings.

### Step 12.3 — Overview

- landing and execution metrics;
- latency and expiration trends;
- unknown/data-quality rates;
- flow/version/route breakdowns;
- baseline selection;
- metric-definition drawers.

### Step 12.4 — Trace search and list

- filter state in URL;
- cursor pagination;
- signature/trace/business-action search;
- accessible state/certainty labels;
- saved filters are P1, not required.

### Step 12.5 — Trace detail

- state summary;
- evidence timeline;
- blockhash validity visualization;
- simulation and submission attempts;
- status observations and execution metadata;
- diagnoses, alternatives, and evidence links;
- recommendations/dispositions;
- missing-evidence checklist;
- related replacement traces.

### Step 12.6 — Comparison and data quality

- cohort builder;
- sample/completeness display;
- absolute/relative descriptive changes;
- small-sample warning;
- instrumentation coverage by app/SDK version.

### Tests

- component behavior with MSW fixtures;
- keyboard navigation and semantic status labels;
- automated accessibility checks for major pages;
- error/loading/empty/incomplete states;
- URL filter persistence;
- trace polling/ETag behavior;
- Playwright happy path and failure investigation;
- screenshot regression for portfolio-critical pages if stable.

### Exit gate

An engineer unfamiliar with the fixture can identify whether it landed, whether execution succeeded, what is known, what is probable, and what is missing without reading raw JSON.

### Portfolio explanation

Shows complex observability UX, typed API integration, accessibility, state management, and technical-information design.

## 20. Phase 13 — Reports, CLI, deletion, and retention

### Goal

Complete audit and operator workflows around the continuous platform.

### Tasks

1. Reuse R0 report model through database/query inputs.
2. Implement report job API and worker.
3. Freeze cohort watermark and rule/schema versions.
4. Implement HTML/JSON artifact storage with size/checksum.
5. Run export-specific redaction and secret scan.
6. Implement CLI:
   - `init`;
   - `doctor`;
   - `trace`;
   - `report create/status/download`;
   - `rules list`;
   - `retention run --dry-run`;
   - `demo`.
7. Implement trace-deletion job and tombstone/audit behavior.
8. Implement partition creation and retention worker.
9. Add backup/restore and incident runbooks.

### Tests

- report retry/idempotency;
- report privacy profiles;
- size cap and checksum;
- report from stable watermark;
- deletion removes all searchable identifiers and derived rows;
- retention safety window/dry-run;
- backup and restore of a fixture deployment;
- CLI human and JSON output;
- tokens/endpoints masked.

### Exit gate

A customer can diagnose in the UI, export a portable audit, operate the installation through the CLI, and apply documented retention/deletion behavior.

### Portfolio explanation

Shows asynchronous report generation, secure exports, CLI UX, lifecycle operations, and privacy deletion.

## 21. Phase 14 — Security and resilience hardening

### Goal

Verify that the product's observability does not become a risk to funds, credentials, availability, or customer strategy.

### Tasks

1. Complete threat-model review against actual code.
2. Run secret fixture matrix through SDK, API, raw DB, projections, logs, UI, and reports.
3. Add malformed/compression/high-cardinality fuzzing.
4. Review authentication and constant-time token verification.
5. Add secure response headers and production CORS policy.
6. Review TLS/reverse-proxy documentation.
7. Add dependency, license, container, and secret scanning to release gate.
8. Force failures:
   - collector down;
   - database down;
   - projector crash;
   - observer rate limit;
   - corrupt RPC response;
   - disk pressure;
   - report worker crash;
   - retention failure.
9. Verify no acknowledged-event loss under supported failure boundary.
10. Review logs for address/signature/privacy policy.
11. Document vulnerability reporting and security limitations.

### Exit gate

All PRD security acceptance scenarios pass, and every critical threat is mitigated, explicitly accepted, or blocks release.

### Portfolio explanation

Shows defense in depth, threat modeling, chaos/failure testing, and awareness that blockchain telemetry can be financially sensitive.

## 22. Phase 15 — Capacity and performance verification

### Goal

Test the system-design estimates rather than repeat them as claims.

### Workload generator

Build deterministic synthetic generation for:

- 1,000 portfolio traces;
- 10,000 small-team traces/day equivalent;
- 50,000 trace / 1M event design dataset;
- 500 event/second burst;
- mixed success/expiry/execution-error distribution;
- route and app-version cardinality;
- observer RPC mock latency/rate limits.

### Benchmarks

1. SDK synchronous overhead and memory.
2. Batch compression ratio and request size.
3. Collector throughput and p95 latency.
4. Database bytes/event and bytes/trace.
5. Projection throughput and lag.
6. Job claim/recovery behavior.
7. Observer batch/RPC rate.
8. Trace-detail query p95.
9. Overview and comparison p95.
10. 100,000-trace report duration.
11. Retention partition drop duration.
12. Container startup and memory.

### Optimization order

1. Measure and inspect query plans/profiles.
2. Fix algorithms and missing indexes.
3. Tune batch sizes and database pool.
4. Add bounded precomputed snapshots only for proven query cost.
5. Do not introduce Redis/Kafka/ClickHouse merely to improve architecture appearance.

### Exit gate

Publish a reproducible benchmark report with hardware, dataset, commands, results, limitations, and any revised capacity claims.

### Portfolio explanation

Shows capacity planning followed by measurement, profiling, SQL plan analysis, and responsible performance claims.

## 23. Phase 16 — Deployment, documentation, and portfolio release

### Goal

Make Landfall easy to evaluate, operate, and discuss with an Upwork client.

### Deployment tasks

1. Multi-stage Rust/server image.
2. Dashboard static build embedded/mounted into server image.
3. PostgreSQL 18 current-minor image pinned by digest for release.
4. Docker Compose profiles for demo and production-like local use.
5. Non-root server container and read-only filesystem where practical.
6. Health checks and graceful shutdown.
7. Example TLS reverse-proxy configuration.
8. Backup/restore scripts or documented commands.
9. Release archive/checksums and image provenance where practical.

### Documentation tasks

- root README focused on problem and five-minute demo;
- architecture overview and diagrams;
- quick start;
- Kit integration guide;
- manual SDK guide;
- privacy field matrix;
- metric definitions;
- diagnostic rule catalog;
- operational runbooks;
- troubleshooting;
- benchmark methodology/results;
- security policy and limitations;
- API documentation;
- contribution guide;
- changelog and release notes.

### Portfolio demo script

The demo must show:

1. Start PostgreSQL and Landfall.
2. Run the controlled Kit example.
3. Observe a successful trace.
4. Trigger a simulation or on-chain execution error.
5. Trigger a client timeout followed by observed success.
6. Load or create an expiration fixture.
7. Compare two app/policy cohorts.
8. Open diagnosis evidence and missing-data explanation.
9. Export a sanitized HTML report.
10. Stop the collector and prove the customer transaction path remains independent.

### Upwork case study

Prepare a concise case study containing:

- client problem archetype;
- architectural diagram;
- difficult engineering decisions;
- safety/privacy constraints;
- performance results;
- screenshots;
- reproducible repository/demo link;
- technologies used;
- what a real customer pilot would measure.

### Exit gate

A technical reviewer can run the demo from a clean machine, understand the architecture from the README, inspect tests, and see measurable evidence rather than a feature-only mockup.

## 24. Cross-cutting test matrix

| Layer | Unit | Integration | Contract | Property/fuzz | End-to-end | Performance |
|---|---:|---:|---:|---:|---:|---:|
| Protocol | Yes | — | Yes | Yes | — | — |
| Core reducer/rules | Yes | — | Fixture | Yes | — | Benchmark |
| Storage/jobs | Yes | Yes/Postgres | — | Concurrency | — | Yes |
| Collector API | Yes | Yes | OpenAPI | Payload fuzz | SDK→API | Yes |
| SDK | Yes | Collector mock/real | Event schemas | Buffer/property | Example | Yes |
| Kit adapter | Yes | Local validator | Supported versions | — | Devnet opt-in | Timing |
| Observer | Yes | RPC mocks/Postgres | Solana response fixtures | Malformed RPC | Controlled network | RPC load |
| Query API | Yes | Postgres | OpenAPI | Filter cases | Dashboard | Query plans |
| Dashboard | Yes | MSW | Generated client | — | Playwright | Bundle/render |
| Reports/CLI | Yes | Postgres | Report schema | Secret fuzz | Full stack | 100k traces |

Coverage percentage is not the sole quality target. Critical invariants and failure boundaries require direct named tests even if line coverage is already high.

## 25. CI/CD plan

### Pull request pipeline

1. Formatting and static checks.
2. Rust unit/property tests.
3. TypeScript unit/component tests.
4. Schema and OpenAPI drift.
5. PostgreSQL repository/migration tests.
6. SDK → collector contract test.
7. Security/license/advisory checks.
8. Build Rust binaries, SDK packages, dashboard, and container.

### Main branch/nightly pipeline

- complete integration suite;
- Playwright full stack;
- local-validator scenarios;
- container scan;
- selected performance regression benchmarks;
- backup/restore smoke test;
- devnet test only with protected secrets and explicit budget, not on untrusted pull requests.

### Release pipeline

- verify clean tag and changelog;
- reproduce all release gates;
- build multi-architecture image and binaries;
- generate SBOM;
- sign/checksum artifacts when tooling is chosen;
- publish npm SDK packages only after package-provenance configuration;
- attach OpenAPI, schemas, reports, and benchmark summary;
- no automatic production deployment in P0.

## 26. Documentation generated during implementation

The following documents evolve with code:

- ADRs;
- event schema reference;
- OpenAPI reference;
- database schema diagram and migration notes;
- rule catalog;
- privacy data inventory;
- support matrix;
- runbooks;
- benchmark report;
- demo guide;
- release checklist;
- Upwork case study.

Documentation is part of phase acceptance, not a final cleanup task.

## 27. Recommended commit/PR slices

Each pull request should be reviewable and teach one concept. Example slices:

1. Workspace and CI skeleton.
2. Event envelope and identifiers.
3. Submission event schema and fixtures.
4. Reducer state dimensions.
5. Expiration diagnostic rule.
6. Offline JSON report.
7. Database control schema.
8. Raw event dedup transaction.
9. Job lease implementation.
10. Collector auth and health.
11. Batch ingestion.
12. Trace projector.
13. SDK context/event API.
14. SDK buffer/transport.
15. Kit adapter.
16. Observer status batcher.
17. Expiration scheduler.
18. Transaction enrichment.
19. Trace query API.
20. Overview metrics.
21. Dashboard trace list/detail.
22. Comparison/data-quality UI.
23. Report worker and CLI.
24. Retention/deletion.
25. Security/performance hardening.
26. Demo/release.

Avoid one enormous “implement backend” PR. Small slices make the work easier to explain, test, and present as professional engineering history.

## 28. Risk-based priorities

### Must prove early

- event semantics can represent real incidents;
- reducer is deterministic under duplicate/out-of-order delivery;
- expiry language is honest;
- raw/private secrets are unnecessary;
- SDK failure is isolated from customer transaction behavior;
- PostgreSQL can atomically persist event and job;
- observer can operate within RPC constraints.

### Can wait

- multiple dashboard themes;
- saved searches;
- alert destinations;
- hosted multi-tenancy;
- enterprise RBAC/SSO;
- Jito bundles;
- WebSocket optimization;
- durable SDK disk spool;
- ClickHouse/Kafka;
- AI-generated summaries.

## 29. Completion criteria

The full implementation plan is complete when the resulting product satisfies all of the following:

1. All P0 PRD acceptance scenarios pass.
2. The event protocol and REST API are versioned and cross-language verified.
3. The domain core deterministically explains golden incidents.
4. Event ingestion is durable, idempotent, bounded, and privacy-enforced.
5. Projection and job recovery survive restart/failure tests.
6. The SDK does not place Landfall on the customer transaction critical path.
7. The observer correctly handles status, block height, expiry, and transaction enrichment.
8. Metrics preserve retry/replacement/business-action semantics.
9. The dashboard communicates evidence and uncertainty clearly.
10. Reports are portable, versioned, and sanitized.
11. Capacity claims are backed by reproducible benchmarks.
12. A clean-machine demo and Upwork case study are available.
13. Every major architectural decision has an ADR.
14. The owner can explain each component, important file, and failure path.

## 30. Immediate next action when implementation is authorized

The first implementation action is **Phase 0, not scaffolding**:

1. Review the 12 open engineering decisions in the system design.
2. Approve or revise the proposed stack in this document.
3. Write ADR-001 through ADR-008 and the support matrix.
4. Only then initialize the workspace in Phase 1.

This prevents apparently harmless bootstrap choices from silently deciding privacy, contract ownership, client compatibility, and persistence behavior.
