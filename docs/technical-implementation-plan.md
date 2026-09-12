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
3. Configure workspace lints, Rust Edition 2024, release profile, and minimal features. Completed: [shared Rust workspace policy](../crates/README.md#shared-workspace-policy).
4. Initialize pnpm workspace, Node 24 pin, TypeScript base config, and package boundaries. Completed: [TypeScript workspace boundaries](../packages/README.md).
5. Create dashboard and package skeletons without product logic. Completed: [TypeScript workspace boundaries](../packages/README.md).
6. Add `.editorconfig`, ignore files, license placeholder/decision, security policy, and contribution basics. Completed: [contribution guide](../CONTRIBUTING.md), [security policy](../SECURITY.md), and [licensing decision](licensing.md).
7. Create `just` commands: `bootstrap`, `fmt`, `lint`, `typecheck`, `test`, `test-integration`, `build`, `check`, `dev`, `db-up`, `db-reset`. Completed: [justfile](../justfile) and [contribution guide](../CONTRIBUTING.md).
8. Add Docker Compose with PostgreSQL health check; server may be a placeholder health binary only when implementation begins. Completed: [Docker Compose configuration](../docker-compose.yml) and [local database workflow](../CONTRIBUTING.md#local-postgresql).
9. Create GitHub Actions jobs. Completed: [CI workflow](../.github/workflows/ci.yml):
   - Rust format/lint/test;
   - TypeScript lint/typecheck/test/build;
   - PostgreSQL integration tests;
   - schema/OpenAPI drift;
   - dependency/license/secret scan;
   - container build.
10. Enable caching without making CI depend on cached artifacts. Completed:
    [cache-backed CI jobs](../.github/workflows/ci.yml) still run locked installs
    and complete verification on cache misses.
11. Add a repository architecture README with links to PRD, design, and plan.
    Completed: [repository overview and architecture guide](../README.md).

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

1. Enumerate P0 event types and common envelope. Completed: [Event Protocol
   Catalog](event-protocol.md).
2. Define JSON Schema for: Completed under
   [`schemas/events/v1`](../schemas/events/v1/).
   - trace created;
   - blockhash acquired;
   - simulation started/completed;
   - signing started/completed;
   - submission started/completed;
   - retry scheduled;
   - confirmation wait started/completed;
   - status observed;
   - execution enriched;
   - business outcome observed;
   - data quality detected.
3. Define decimal-string rules for values beyond JavaScript safe integer range.
   Completed: [Decimal-String Contract](decimal-string-rules.md).
4. Define enums for privacy mode, commitment, transport result, RPC result, and
   normalized errors. Completed: [Event Enum Contract](event-enum-contract.md).
5. Implement Rust wire types in `landfall-protocol`. Completed in
   [`crates/landfall-protocol`](../crates/landfall-protocol/).
6. Generate or verify `protocol-ts` types. Completed with the committed
   [`v1.ts`](../packages/protocol-ts/src/generated/v1.ts) artifact and
   deterministic generator drift check.
7. Create valid and invalid fixture corpus. Completed under
   [`fixtures/protocol/v1`](../fixtures/protocol/v1/) with shared JSON Schema,
   Rust Serde, and TypeScript/Node verification.
8. Add schema version compatibility rules. Completed with the
   [exact-version capability policy](schema-version-compatibility.md), shared
   compatibility fixtures, and matching Rust/TypeScript checks.
9. Add redaction fixtures with keys, bearer tokens, RPC URLs, cookies, and oversized metadata.
   Completed under [`fixtures/protocol/privacy`](../fixtures/protocol/privacy/)
   with structural-rejection and canonical-redaction cases shared by Rust and
   TypeScript/Node checks.
10. Document examples and field-level privacy classification.
    Completed in [Event examples and field-level privacy
    classification](event-privacy-classification.md), backed by a
    machine-readable registry and schema-drift check.

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
   Completed in [`landfall-core::domain`](../crates/landfall-core/src/domain/)
   with typed UUIDv7 and source-derived IDs, non-empty deduplicated evidence,
   partial lifecycle evidence, and checked business-action/trace boundaries.
2. Define orthogonal state dimensions:
   - lifecycle stage;
   - landing state;
   - execution result;
   - application outcome;
   - observation completeness.
   Completed in [`domain/state.rs`](../crates/landfall-core/src/domain/state.rs)
   with a validated `TraceState` product type and explicit terminal-metric
   eligibility without collapsing network, execution, and application claims.
3. Implement canonical event ordering with clock-quality warnings.
   Completed in [`landfall-core::ordering`](../crates/landfall-core/src/ordering/)
   with semantic and comparable-monotonic constraints, stable wall/receive/ID
   fallback, duplicate handling, and explicit clock-quality warnings.
4. Implement the pure trace reducer.
   Completed in [`landfall-core::reducer`](../crates/landfall-core/src/reducer/)
   with full-replay identity validation, independently derived lifecycle/landing/
   execution/application/completeness dimensions, expiry inference, evidence
   retention, explicit ambiguity warnings, and a stable reducer version.
5. Implement retry versus replacement grouping rules.
   Completed in [`landfall-core::grouping`](../crates/landfall-core/src/grouping/)
   with attempt-ID grouping, retry-intent separation, explicit business-action
   replacement links, environment-scoped signature/fingerprint comparison,
   alias-candidate classification, and fail-closed identity conflicts.
6. Implement data-quality grading.
   Completed in [`landfall-core::data_quality`](../crates/landfall-core/src/data_quality/)
   with the documented [`data-quality-v1`](data-quality-grading.md) rubric,
   contextual required/optional evidence checks, deduplicated evidence-linked
   findings, privacy-aware gaps, and deterministic A–F scoring.
7. Implement initial confirmed rules:
   - simulation error;
   - RPC rejection;
   - on-chain execution error;
   - compute-budget failure;
   - validity window passed without observed inclusion;
   - client timeout followed by network success.
   Completed in [`landfall-core::diagnostics`](../crates/landfall-core/src/diagnostics/)
   with the documented [`diagnostic-rules-v1`](diagnostic-rule-catalog.md)
   confirmed rule set, stable rule IDs, per-claim certainty, evidence-linked
   findings, compute-budget specificity, expiration evidence selection, and
   deterministic replay tests.
8. Implement initial probable rules:
   - excessive signing delay;
   - low compute headroom;
   - route degradation signal;
   - unsafe/redundant retry;
   - fee likely uncompetitive only with fee-market evidence.
   Completed in [`landfall-core::diagnostics`](../crates/landfall-core/src/diagnostics/)
   with configurable conservative thresholds, probable certainty, explicit
   evidence gating, and trace-level golden tests. Fee diagnosis remains silent
   until the required comparable fee observations are available.
9. Implement unknown and missing-evidence generation.
   Completed in [`landfall-core::diagnostics`](../crates/landfall-core/src/diagnostics/)
   with `RULE-UNKNOWN-001`, explicit reason codes, `Unknown` certainty,
   immutable event anchors, and tests proving that evidence gaps do not become
   causal failure claims.
10. Implement advisory recommendations and evidence references.
    Completed in [`landfall-core::recommendations`](../crates/landfall-core/src/recommendations.rs)
    with versioned recommendation keys, diagnostic links, immutable evidence
    propagation, and advisory-only behavior.
11. Implement metric definitions as pure functions over projections.
    Completed in [`landfall-core::metrics`](../crates/landfall-core/src/metrics.rs)
    with versioned terminal-denominator, landing, execution, and application
    flags plus deterministic eligibility helpers.
12. Version reducer, rule set, and metric definitions.
    Completed in [`landfall-core::versions`](../crates/landfall-core/src/versions.rs)
    with a single immutable manifest and semantic-compatibility check covering
    reducer, diagnostics, metrics, and recommendations.
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
   Completed in `landfall-cli` with streaming line-by-line decoding, semantic
   validation, stdin support via `-`, stable JSON count output, and line-aware
   errors.
2. Stream and validate NDJSON rather than load the entire file.
3. Group events into canonical traces and aliases in memory.
   Completed in `landfall-cli` with trace partitioning, canonical ordering,
   in-memory `TraceGrouping` reconstruction, and pairwise alias/replacement
   classification within project/environment boundaries.
4. Run reducer, data quality, diagnoses, recommendations, and metrics.
   Implemented by the CLI in one in-memory trace analysis pipeline, preserving
   each product and its evidence links for the subsequent report model.
5. Build a stable report-domain model.
   Implemented in `landfall-report` with presentation-neutral `ReportDocument`,
   `TraceReport`, semantic version manifest, stable state tokens, and pipeline
   count fields.
6. Render structured JSON.
   Implemented using `serde` on the report-domain model; CLI emits a stable
   pretty-printed `ReportDocument` containing versions, trace summaries, and
   alias counts.
7. Render self-contained HTML with evidence timeline and limitations.
   Implemented in `landfall-report::render_html` with embedded CSS, stable state
   and diagnostic columns, semantic versions, and no external assets.
8. Add `--privacy-profile` export redaction.
   Implemented with `internal` and `shareable` profiles for JSON and HTML;
   shareable exports redact trace identifiers while preserving analytical data.
9. Add example input and expected report snapshots.
   Added deterministic offline snapshot coverage in `landfall-cli` and the
   [`offline-golden-incidents`](offline-golden-incidents.md) runbook.
10. Document how to inspect each golden incident.
    Added [`golden-incident-runbook.md`](golden-incident-runbook.md) with
    copy-paste CLI commands, fixture-to-behavior mapping, and privacy-profile
    guidance.

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
   Completed with append-only migration `0001_create_schemas.sql`, baseline
   metadata/batch/job/report tables, and PostgreSQL smoke validation.
3. Implement projects, environments, routes, and API-token tables.
   Completed in migration `0002_create_control_entities.sql` with scoped
   foreign keys, uniqueness boundaries, route enablement, token expiry/revocation,
   and hash-only token storage.
4. Implement event dedup registry and daily raw-event partition parent.
   Completed in migration `0003_create_telemetry_events.sql` with an immutable
   event identity registry, range-partitioned raw event parent, and bounded
   environment/trace time indexes without a JSON GIN index.
5. Implement business actions, traces, aliases, typed child projections.
   Completed in migration `0004_create_reporting_entities.sql` with scoped
   business-action/trace tables, explicit alias relationships, and typed
   attempt, simulation, observation, and execution projections.
6. Implement diagnostics, evidence, recommendations, and disposition history.
   Completed in migration `0005_create_diagnostics_recommendations.sql` with
   versioned certainty, many-to-many evidence links, recommendation-to-
   diagnostic links, and append-only disposition records.
7. Implement jobs with lease fields and active dedupe keys.
   Completed in migration `0006_create_work_jobs.sql` with explicit lifecycle
   states, `locked_by`/`locked_until` leases, retry counters, and a partial
   unique index for ready/running dedupe keys.
8. Implement report metadata/artifact tables.
   Completed in migration `0007_create_report_artifacts.sql` with scoped report
   metadata and immutable JSON/HTML artifact manifests containing privacy,
   storage-key, hash, and byte-size attributes.
9. Build repository traits at service boundaries and SQLx implementations.
   Completed with `PersistenceRepository`, `PgRepositories`, and a translated
   `RepositoryError` boundary; SQL remains behind the trait.
10. Implement migration runner and version health check.
   Completed with compile-time embedded SQLx migrations, an explicit runner,
   and a health result that detects failed, missing, or unexpected versions.
11. Implement test database lifecycle and seed helpers.
   Completed with explicit PostgreSQL reset and deterministic control-plane
   seed helpers that preserve the SQLx migration ledger.
12. Add partition creation/retention primitives.
   Completed with date-scoped daily partition creation and explicit retention
   cleanup that validates generated identifiers before executing DDL.
13. Inspect query plans for trace lookup and bounded time search.
   Completed with a Docker PostgreSQL smoke script covering trace and raw-event
   bounded queries and asserting the intended composite indexes.

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
   Completed with typed application state, `/v1/ingest`, bounded batches, and
   structured `400`/`413` validation responses.
2. Add request IDs and structured tracing spans.
   Completed with propagated/generated `X-Request-Id` headers and an
   `http_request` span containing request ID, method, and path.
3. Add liveness/readiness endpoints.
   Completed with `/health/live` for process liveness and `/health/ready` for
   dependency readiness, including a `503` response when not ready.
4. Implement bearer-token hashing, lookup, scope, expiration, and revocation.
   Completed with hash-only SHA-256 token records and constant-time lookup
   policy covering bearer parsing, scope, expiry, and revocation decisions.
5. Implement compressed and decompressed body limits.
   Completed with an early compressed `Content-Length` guard and Axum's
   decompressed request-body limit for JSON extraction.
6. Implement batch schema/semantic validation.
   Completed with schema-version/event-type compatibility checks and atomic
   rejection of batches containing unsupported or malformed events.
7. Repeat privacy enforcement server-side.
   Completed with recursive prohibited-key scanning across objects and arrays,
   returning a non-reflective `privacy_violation` error before persistence.
8. Implement atomic transaction:
   - dedup claim;
   - raw-event insert;
   - minimal identity upsert;
   - projection job enqueue.
   Completed with `ingest_atomically`, which commits batch metadata, dedup
   claims, raw events, and trace projection jobs as one transaction.
9. Implement `202`, duplicate `200`, and structured error responses.
   Completed with a centralized response mapper: new durable events return
   `202`, fully duplicate batches return `200`, and validation failures retain
   stable structured error codes.
10. Implement token/request rate and concurrency limits.
   Completed with a bounded process-wide request window (`429` on exhaustion)
   and Axum/Tower concurrency limiting before handler execution.
11. Generate OpenAPI route and schema snapshot.
   Completed with a code-first `utoipa` document exposed at `/openapi.json`
   and a snapshot-style contract test.
12. Add disposable health-check event endpoint.
   Completed with `POST /health/event`, which returns a clearly marked
   synthetic event and does not persist customer telemetry.

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
   Completed with a tracked `WorkerSupervisor`, child cancellation tokens, and
   graceful shutdown awaiting every worker handle.
2. Implement `project_trace` job worker.
   Completed with a cancellable bounded-queue worker loop and injected
   processor callback for deterministic orchestration tests.
3. Resolve trace aliases by signature/digest.
   Completed with a project/environment-scoped alias index for versioned
   fingerprints; cross-environment matches are intentionally unrelated.
4. Load canonical raw events.
   Completed with an environment/project-scoped bounded time-range query,
   ordered deterministically by occurred/received timestamps and event ID.
5. Run deterministic reducer.
   Completed with a persisted-row adapter that deserializes `WireEvent`,
   canonical-orders events, and invokes the existing pure reducer.
6. Replace/upsert typed child projections in one transaction.
   Completed with transactional trace upsert and delete/replace of typed
   submission-attempt children in a single SQLx transaction.
7. Preserve/supersede diagnosis history.
   Completed with append-only versioned diagnostic writes and evidence links;
   newer rows supersede prior claims by rule-set version without mutating history.
8. Persist recommendation and evidence links.
   Completed with atomic append-only recommendation writes and duplicate-safe
   links to diagnostics and immutable evidence events.
9. Increment projection version/watermark.
   Completed with a dedicated per-trace watermark table and monotonic
   version advance helper that ignores stale projection writes.
10. Enqueue observation only when eligible.
   Completed with an explicit eligibility gate and deduplicated `observe_trace`
   job type; ineligible traces enqueue nothing.
11. Add projection-lag and dead-job metrics.
   Completed with atomic in-process counters for projected, failed, and dead
   jobs plus a gauge-like projection lag snapshot.
12. Add reproject command for one trace, time range, or rule version.
   Completed with validated CLI selector parsing for trace, bounded RFC3339
   range, or rule-set version replay scopes.

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
   Completed with `LandfallSdk`, `TraceContext`, `BusinessActionContext`, and
   a callback-based telemetry error policy that never throws from `emit`.
2. Implement validated immutable configuration.
   Completed with URL, batch-size, and buffer-size validation plus a frozen
   `SdkConfig` snapshot exposed read-only by the SDK.
3. Implement `BusinessActionContext` and `TraceContext`.
   Completed with validated immutable business-action contexts and explicit
   trace linkage for grouping multiple transaction attempts.
4. Implement typed manual lifecycle event builders.
   Completed with typed builders for trace creation, signing start, and
   submission start using generated protocol envelopes.
5. Implement UUIDv7 generation/canonical format.
   Completed with a canonical lowercase UUIDv7 generator and format validator
   for SDK identities.
6. Implement signed-byte fingerprinting according to ADR.
   Completed with a bytes-only fingerprint helper using injected HMAC and
   protocol-compatible 32-byte lowercase hex output.
7. Implement endpoint redaction and metadata allowlist.
   Completed with endpoint credential/query redaction and bounded scalar
   metadata allowlisting for explicitly approved keys.
8. Implement bounded in-memory buffer.
   Completed with a generic FIFO buffer enforcing a fixed capacity and
   explicit overflow result for health/error accounting.
9. Implement batch assembly and stable retries.
   Completed with bounded batch assembly, generated batch IDs, and retry
   objects preserving the original ID and event ordering.
10. Implement HTTP/gzip transport and backoff with jitter.
   Completed with an injected batch transport, gzip flag, bounded retry count,
   exponential backoff, and injectable jitter/sleep for deterministic tests.
11. Implement drop/transport health counters and customer callback.
   Completed with aggregate dropped-event and transport-failure counters,
   immutable snapshots, and a fail-open health-change callback.
12. Implement bounded shutdown flush.
   Completed with a timeout-bounded flush helper and SDK failure reporting;
   shutdown cannot wait indefinitely for a collector.
13. Build ESM package and declarations; add CJS only if support decision requires it.
   Completed with an ESM-only package export, explicit module/types metadata,
   and declaration generation verified by a package smoke test.
14. Create manual-events example.
   Completed with `examples/sdk-manual-events.mjs`, demonstrating typed
   lifecycle builders, bounded buffering, batch assembly, and shutdown flush.

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
   Completed with a machine-checked lane manifest for exact Kit 8.2.0, Node
   24 compatibility, and an explicit empty plugin tuple.
2. Define adapter boundary independent of Kit internals.
   Completed with generic `SolanaClientPort` lifecycle types and a
   result/error-preserving customer-operation wrapper; the neutral SDK has no
   `@solana/kit` runtime import.
3. Capture blockhash and `lastValidBlockHeight`.
   Completed with boundary normalization preserving exact decimal block-height
   precision (including Kit bigint values) and rejecting non-canonical input.
4. Capture simulation result and compute units when available.
   Completed with client-neutral simulation normalization, exact compute-unit
   serialization, log presence detection, and stable error classification.
5. Measure signing delay without accessing signer secrets.
   Completed with an injected monotonic clock and a signing-operation wrapper
   that records exact nanosecond duration while preserving value/error identity.
6. Fingerprint signed bytes in memory.
   Completed with a transient-copy fingerprint helper that passes exact bytes
   to the injected HMAC and wipes the copy immediately after hashing.
7. Wrap submission attempts with route and configuration.
   Completed with a route-aware submission wrapper carrying stable attempt IDs,
   sequence, encoding, and preflight settings while preserving outcomes.
8. Capture application confirmation-wait result.
   Completed with monotonic wait-duration capture and explicit commitment,
   timeout, cancellation, and failure classification.
9. Build a controlled SOL-transfer or harmless-instruction example for local validator/devnet.
   Completed with a 1,000-lamport dry-run harness, explicit local/Devnet
   selection, and an opt-in submission gate.
10. Demonstrate success, simulation failure, client timeout/later success, and expiration fixtures.
   Completed with four JSON lifecycle fixtures and a validation script covering
   late landing after client timeout and the exact post-validity expiration boundary.
11. Document the integration and every captured field.
   Completed with the lifecycle contract, field-level source/representation/
   privacy table, and explicit Kit-versus-neutral-SDK responsibilities.
12. After the Kit lane is stable, run a time-boxed `@solana/web3.js` v3 compatibility spike; defer production support and a legacy v1 adapter until evidence justifies them.
   Completed as an 8-hour, evidence-driven spike plan. Production support and
   a legacy v1 adapter remain explicitly deferred.

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
   Completed with an injected Rust transport, JSON-RPC 2.0 request/response
   handling, and normalized transport/provider/malformed-response errors.
2. Implement route-specific pooled Reqwest client with Rustls.
   Completed with a route-owned pooled `reqwest::Client`, Rustls-only TLS,
   bounded timeout, endpoint consistency check, and sanitized HTTP failures.
3. Implement `getBlockHeight` shared cache/worker.
   Completed with a route-scoped TTL cache that shares fresh heights across
   observer jobs and refreshes through the provider-neutral JSON-RPC client.
4. Implement durable observation schedule and in-memory priority queue.
   Completed with a deterministic due-time/priority queue; durable job rows can
   be re-enqueued after restart without changing ordering semantics.
5. Batch `getSignatureStatuses` by route/options, default 100, maximum 256.
   Completed with bounded 256-signature chunks and stable result ordering
   across multiple JSON-RPC calls.
6. Implement adaptive polling and per-route rate limits.
   Completed with bounded exponential polling intervals, rate-limit expansion,
   and independent per-route minimum request intervals.
7. Persist first null, state changes, periodic checkpoints, RPC error transitions, and terminal evidence.
   Completed with an append-only evidence log covering first-null, status
   transitions, checkpoints, RPC error transitions, and terminal outcomes.
8. Implement expiry evaluation from current block height and `lastValidBlockHeight`.
   Completed with strict `current > last_valid` evaluation and an explicit
   durable-nonce indeterminate result.
9. Enqueue and implement `getTransaction` enrichment.
   Completed with a provider-neutral request that explicitly advertises
   `maxSupportedTransactionVersion: 0` and safely preserves a missing result.
10. Normalize execution error, fee, compute units, slot, block time, and transaction version.
   Completed with provider-neutral execution normalization, exact decimal
   integer fields, version mapping, log-presence signal, and error flag.
11. Implement observer route health and data-quality gaps.
   Completed with per-route request/success/failure/rate-limit counters and
   explicit gap categories for missing or unusable evidence.
12. Detect durable-nonce/unsupported validity cases without false expiration.
   Completed with explicit validity classification and separate indeterminate /
   unsupported outcomes that bypass recent-blockhash expiry rules.
13. Support clean cancellation and restart recovery.
   Completed with cooperative `CancellationToken` shutdown and deterministic
   rehydration of durable schedules into a fresh in-memory queue.

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
   Completed with bounded opaque offset cursors, deterministic slice pagination,
   and explicit invalid-cursor/limit errors.
2. Implement trace list filters and bounded-time validation.
   Completed with project/environment/status filters and RFC3339 time windows
   bounded to a maximum of 31 days.
3. Implement trace detail with attempts, observations, evidence, recommendations, and watermark.
   Completed with a generic serializable trace-detail read model carrying all
   five sections and an explicit projection watermark.
4. Implement signature lookup with privacy/log safeguards.
   Completed with strict-mode denial, empty-input validation, and a bounded
   response carrying only signature/status/log-presence metadata.
5. Implement business-action detail and multiple-success warning.
   Completed with a serializable business-action read model grouping trace IDs
   and flagging more than one successful trace.
6. Implement metric summary with numerator/denominator/exclusions.
   Completed with versioned summaries that exclude in-flight and unknown rows
   explicitly rather than silently treating them as failures or successes.
7. Implement data-quality summary.
   Completed with deterministic grade distribution, gap counts, explicit empty
   selection semantics, and the scoring-rubric version.
8. Implement descriptive cohort comparison.
   Completed with completed-window safeguards, sample-size warnings, missing
   data rates, and absolute/relative metric changes.
9. Implement recommendation disposition and history.
   Completed with append-only typed decisions, bounded reasons, actor/timestamp
   audit fields, latest-state lookup, and duplicate-ID protection.
10. Implement project/environment/route read configuration endpoints.
    Completed with deterministic redacted project/environment/route payloads;
    secrets, credentials, and raw endpoints are never exposed.
11. Implement authenticated detailed system status.
    Completed with `system:read` authorization, redacted dependency metrics,
    queue/lag visibility, observer health, retention, and version reporting.
12. Add ETag/projection version behavior.
    Completed with strong version-derived ETags, projection-version metadata,
    and exact `If-None-Match` conditional-read semantics.
13. Generate OpenAPI and TypeScript API client.
    Completed by registering Phase 11 schemas in `ApiDoc` and adding the typed
    `@landfall/api-client` fetch client with injectable transport.

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

Completed with the React/Vite shell, hash-based overview/traces/comparison
routes, design tokens, responsive accessible navigation, error boundary, and a
fixture-mode loading surface. API-client/MSW wiring remains isolated for the
following dashboard tasks.

### Step 12.2 — Onboarding and system health

- project/environment status;
- first-trace checklist;
- collector/observer/schema/privacy display;
- CLI command hints;
- data-quality warnings.

Completed with an onboarding health panel, first-trace CLI checklist, project
and environment status, collector/observer/schema/privacy indicators, and an
explicit incomplete-observer data-quality warning.

### Step 12.3 — Overview

- landing and execution metrics;
- latency and expiration trends;
- unknown/data-quality rates;
- flow/version/route breakdowns;
- baseline selection;
- metric-definition drawers.

Completed with fixture-backed operational metric cards, baseline deltas, a
latency p95 trend, route coverage breakdown, and an accessible metric-definition
action placeholder.

### Step 12.4 — Trace search and list

- filter state in URL;
- cursor pagination;
- signature/trace/business-action search;
- accessible state/certainty labels;
- saved filters are P1, not required.

Completed with an accessible fixture trace list, free-text filtering for trace
and route identifiers, URL-hash filter persistence, semantic status labels,
empty-state handling, and bounded next-page control.

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

Completed with fixture trace detail navigation, lifecycle state summary,
attempt/observation evidence, execution metadata, diagnosis and recommendation
sections, and an explicit missing-evidence checklist.

### Step 12.6 — Comparison and data quality

- cohort builder;
- sample/completeness display;
- absolute/relative descriptive changes;
- small-sample warning;
- instrumentation coverage by app/SDK version.

Completed with a version-based cohort builder, sample and completeness
display, descriptive absolute/relative changes, small-sample warning, and
instrumentation coverage by SDK and collector schema.

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

Manual startup and verification steps are documented in
[`dashboard-runbook.md`](dashboard-runbook.md).

## 20. Phase 13 — Reports, CLI, deletion, and retention

### Goal

Complete audit and operator workflows around the continuous platform.

### Tasks

1. Reuse R0 report model through database/query inputs.
   Completed with a query-row adapter that reuses the R0 trace projection and
   carries a frozen projection watermark in `ReportSnapshot`.
2. Implement report job API and worker.
   Completed with a bounded report queue, registry-backed status transitions,
   frozen watermark capture, failure details, and cooperative cancellation.
3. Freeze cohort watermark and rule/schema versions.
   Completed with immutable `FrozenReportScope` validation capturing cohort,
   projection watermark, event schema, and core semantic versions.
4. Implement HTML/JSON artifact storage with size/checksum.
   Completed with format/privacy metadata, a 10 MiB size cap, SHA-256
   integrity checks, and immutable download bytes.
5. Run export-specific redaction and secret scan.
   Completed with recursive sensitive-key/value scanning and shape-preserving
   `[redacted]` export transformation.
6. Implement CLI:
   - `init`;
   - `doctor`;
   - `trace`;
   - `report create/status/download`;
   - `rules list`;
   - `retention run --dry-run`;
   - `demo`.
   Completed with a validated command dispatcher, human/JSON output, and the
   existing NDJSON ingest path preserved.
7. Implement trace-deletion job and tombstone/audit behavior.
   Completed with validated deletion jobs, immutable tombstones, audit reason
   and actor fields, and read-path deletion checks.
8. Implement partition creation and retention worker.
   Completed with deterministic cutoff planning, sorted partition selection,
   safety validation, and explicit dry-run semantics.
9. Add backup/restore and incident runbooks.
   Completed with checksum- and schema-validated backup manifests plus a
   documented isolated-restore and projection-replay procedure.

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
   Completed with a control-to-code matrix and explicit open runtime/release
   gaps tracked for later Phase 14 tasks.
2. Run secret fixture matrix through SDK, API, raw DB, projections, logs, UI, and reports.
   Completed with a reusable cross-surface scanner and clean/secret fixture
   tests that report surface names without echoing secret values.
3. Add malformed/compression/high-cardinality fuzzing.
   Completed with panic-free structural fuzz guards for nesting, arrays, and
   object/label cardinality, complementing existing compressed-body limits.
4. Review authentication and constant-time token verification.
   Completed with strict bearer parsing, SHA-256 hash-only records,
   constant-time digest comparison, and negative lifecycle/scope tests.
5. Add secure response headers and production CORS policy.
   Completed with restrictive browser security headers and exact same-origin
   CORS validation that rejects wildcard/cross-site origins.
6. Review TLS/reverse-proxy documentation.
   Completed with a deployment review covering TLS termination, forwarded-header
   trust, edge limits, log scrubbing, private upstream binding, and PostgreSQL
   isolation.
7. Add dependency, license, container, and secret scanning to release gate.
   Completed with `scripts/release-security-gate.sh`, combining lockfile and
   image metadata checks, Gitleaks, and approved Node license scanning.
8. Force failures:
   - collector down;
   - database down;
   - projector crash;
   - observer rate limit;
   - corrupt RPC response;
   - disk pressure;
   - report worker crash;
   - retention failure.
   Completed with an explicit fault-injection scenario matrix and safe outcome
   contract covering retry, halt, rehydrate, quarantine, and dry-run paths.
9. Verify no acknowledged-event loss under supported failure boundary.
   Completed with an explicit acknowledged-vs-durable ID comparison that
   reports sorted gaps and fails closed on any missing event.
10. Review logs for address/signature/privacy policy.
   Completed with a category-only log scanner, representative leak corpus, and
   an operational review procedure that never copies sensitive values.
11. Document vulnerability reporting and security limitations.
   Completed with repository-level `SECURITY.md` covering private reporting,
   triage evidence, disclosure expectations, threat-model scope, and explicit
   non-goals/operational responsibilities.

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

Step 15.1 completed with a seeded synthetic trace generator and explicit burst
target helper. Generated workloads are deterministic and include mixed outcome,
route, and application-version cardinality.

### Benchmarks

1. SDK synchronous overhead and memory.
   Step 15.2 completed with a repeatable Node harness reporting capture time and
   heap delta as JSON, including explicit runtime/benchmark limitations.
2. Batch compression ratio and request size.
   Step 15.3 completed with deterministic 100/1,000-event gzip measurements of
   raw and compressed request sizes.
3. Collector throughput and p95 latency.
   Step 15.4 completed with an in-process parser benchmark reporting events/sec
   and p95 latency, with network/database limitations documented.
4. Database bytes/event and bytes/trace.
   Step 15.5 completed with normalized storage-size measurement helpers and
   explicit denominator/version/index-footprint recording requirements.
5. Projection throughput and lag.
   Step 15.6 completed with normalized events/sec and watermark-lag helpers,
   including empty-workload and non-negative lag safeguards.
6. Job claim/recovery behavior.
   Step 15.7 completed with lease-based claim, expiry recovery, attempt
   counting, and terminal completion semantics.
7. Observer batch/RPC rate.
   Step 15.8 completed with normalized calls/sec, calls-per-batch, and
   rate-limited-ratio metrics plus invalid-counter safeguards.
8. Trace-detail query p95.
   Step 15.9 completed with deterministic p50/p95/max query-latency summaries
   and explicit empty-run safeguards.
9. Overview and comparison p95.
   Step 15.10 completed with independent percentile summaries for overview and
   comparison query classes and workload metadata requirements.
10. 100,000-trace report duration.
   Step 15.11 completed with large-cohort duration normalization and required
   format/privacy/watermark benchmark metadata.
11. Retention partition drop duration.
   Step 15.12 completed with partitions/second normalization, explicit
   dry-run/actual mode, and a runbook describing PostgreSQL/storage metadata
   and the limitation that the helper does not execute destructive SQL.
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
   Step 16.1 verified the existing pinned builder/runtime image, non-root
   runtime user, locked release build, and Dockerfile lint command; details are
   in [deployment-image.md](deployment-image.md).
2. Dashboard static build embedded/mounted into server image.
   Step 16.2 adds a dedicated Node/pnpm dashboard-builder stage and copies only
   Vite output into `/opt/landfall/dashboard`; runtime serving remains a
   separate integration concern documented in [dashboard-image.md](dashboard-image.md).
3. PostgreSQL 18 current-minor image pinned by digest for release.
   Step 16.3 verified the existing PostgreSQL 18.6 Bookworm digest pin and
   documented the update/release procedure in [postgres-image.md](postgres-image.md).
4. Docker Compose profiles for demo and production-like local use.
   Step 16.4 adds `demo` and `production` profiles with isolated named volumes;
   usage and validation are documented in [compose-profiles.md](compose-profiles.md).
5. Non-root server container and read-only filesystem where practical.
   Step 16.5 verifies the image's UID/GID 65532 runtime user and documents the
   `--read-only` plus constrained `/tmp` deployment profile in
   [container-hardening.md](container-hardening.md).
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
