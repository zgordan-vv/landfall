# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Landfall is a self-hosted observability and diagnostic system for the lifecycle of Solana transactions. It correlates application-side evidence (build, simulate, sign, submit, retry) from a TypeScript SDK with RPC responses and on-chain observations, and produces timelines, evidence-linked diagnoses, metrics, and recommendations with explicit confidence (confirmed / probable / unknown). The UI and API must never fabricate data: when the API or database is unavailable, they show the error.

## Toolchain (pinned, enforced)

Rust 1.98.0 (`rust-toolchain.toml`), Node 24.20.0 (`.node-version`), pnpm 11.25.0, just 1.58.0, Docker Compose v2. pnpm is configured to fail on version or engine mismatches (`devEngines`, `engineStrict`), so use the exact versions.

## Commands

All main workflows go through `just` (run `just` to list recipes):

```sh
just bootstrap        # check toolchain, cargo fetch --locked, pnpm install --frozen-lockfile
just check            # full local/CI gate: fmt-check, lint, typecheck, test, build
just fmt              # cargo fmt + prettier
just lint             # clippy -D warnings, Rust dependency-direction check, TS boundary check + tsc
just test             # cargo test --workspace + pnpm test
just db-up            # start PostgreSQL 18.6 (no published host port) and verify invariants
just dev              # Vite dashboard dev server
just container-build  # build and inspect the landfall-server image
just generate-protocol  # regenerate TS wire DTOs from schemas/events/v1
```

To run a single test:

```sh
cargo test -p landfall-core --test trace_reducer            # one Rust integration-test file
cargo test -p landfall-core --test trace_reducer some_name  # one test by name filter
node --test packages/sdk-ts/tests/<file>.test.mjs           # SDK tests run against built dist/, so build first
pnpm --filter @landfall/sdk test
```

Protocol and contract checks: `just check-protocol-fixtures`, `just check-protocol-compatibility`, `just check-protocol-privacy`, `pnpm run check:protocol-generated`, `just check-contracts`.

A destructive DB reset requires `LANDFALL_CONFIRM_DB_RESET=YES just db-reset`.

## Architecture

Deployment is a modular monolith: one Rust `landfall-server` container (HTTP API, projection workers, Solana observer, compiled dashboard) and one PostgreSQL container. There is no Kafka, Redis, or other broker; background work uses a PostgreSQL job queue (`work.jobs`) with leasing.

**Data flow:** SDK → `POST /v1/ingest` → immutable `telemetry.raw_events` and jobs are written atomically → projector workers lease jobs and idempotently build relational read models in the `reporting.*` schema (tolerating duplicate or out-of-order events) → the observer (`landfall-server worker` subcommand) adds RPC evidence → versioned rules in `landfall-core` produce diagnostics and recommendations → query APIs serve the dashboard and CLI. Raw events are the source of truth and must stay replayable.

**Rust crate dependency direction** (enforced by `scripts/check-rust-dependency-direction.sh`):

```
protocol ← core ← {storage, observer, report} ← server ← cli
```

- `landfall-protocol` holds the wire types and depends on no other Landfall crate.
- `landfall-core` holds pure deterministic domain logic (reducer, ordering, grouping of retries and replacements, diagnostics, recommendations, data quality). It must not pull in HTTP, SQL, or Solana client deps. This check scans its full dependency tree.
- `storage`, `observer`, and `report` are sibling adapters that never depend on one another.
- `landfall-server` is the composition root (axum router in `src/lib.rs`, `/v1/*` authenticated routes plus read-only `/demo/v1/*` routes).
- Adding a crate or an exception requires an ADR update and a change to the expected graph in the check script in the same commit.

**Workspace lint policy** (root `Cargo.toml`): edition 2024, `unsafe` forbidden, clippy `pedantic`, and `unwrap`/`expect`/`panic!`/`todo!`/debug macros are denied (CI relaxes unwrap/expect only for its clippy run, but `just lint` does not). Missing public docs are linted. Every crate has an empty `default` feature set. Release builds keep overflow checks on and use `panic = abort`.

**Database:** SQL migrations live in `migrations/` and are embedded with `sqlx::migrate!` in `landfall-storage/src/migrations.rs`. Schemas: `control`, `identity`, `telemetry`, `reporting`, `work`. Config comes from `DATABASE_URL_FILE` (preferred) or `DATABASE_URL`. When you add a table, also update `reset_test_data` in `landfall-storage/src/test_support.rs`.

**Event contract:** `schemas/events/v1/` (JSON Schema) is canonical (ADR 005). `packages/protocol-ts` contains generated DTOs; regenerate them and never hand-edit. Rust types in `landfall-protocol` must match, which the fixture corpus in `fixtures/protocol/` (valid, invalid, and privacy redact/reject cases) checks from both Rust tests and Node/Ajv scripts. Change schemas, generated types, Rust types, and fixtures together. Breaking event or REST changes need an explicit versioning decision. OpenAPI is code-first via Utoipa (ADR 008).

**TypeScript workspace** (pnpm; `apps/*`, `packages/*`, `examples/*`): `protocol-ts ← sdk-ts`, `protocol-ts ← api-client ← apps/dashboard`. Use `workspace:*` deps and public `exports` only: no cross-package `src` imports and no `paths` aliases (checked by `scripts/check-typescript-dependency-direction.mjs`). The dashboard talks only to the API through `@landfall/api-client` and has no fixture mode. It is currently mostly one large file, `apps/dashboard/src/main.tsx`. `tsconfig.base.json` is very strict (`exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`, `isolatedDeclarations`).

## Constraints

- Privacy is a core invariant (ADR 004, `docs/security/`, `docs/threat-model.md`). Never put private keys, seed phrases, signed transaction bytes, bearer tokens, credential-bearing RPC URLs, or real identifiers into code, fixtures, tests, or logs. The SDK and protocol reject or redact these fields.
- `.landfall/` is gitignored and holds local deployment credentials and tokens. Do not read it, print it, or commit it.
- Decisions that are hard to reverse or that cross trust boundaries need an ADR (`docs/adr/000-template.md`; the index is `docs/adr/README.md`).
- Recent commit style is Conventional Commits (`fix: ...`).
