# Upwork case study: Landfall

## Client problem

Solana teams often know that a transaction failed, but cannot explain whether
the cause was application construction, signing latency, RPC routing, retry
behavior, validity expiration, or on-chain execution. Explorers show only the
final chain perspective; application logs rarely preserve a replayable,
privacy-safe lifecycle.

## Solution

Landfall is a self-hosted transaction observability and diagnostic system. A
TypeScript SDK records allowlisted application evidence, a Rust server stores
immutable events in PostgreSQL, projectors build typed read models, and an
observer adds RPC evidence. The dashboard, CLI, and reports consume the same
query contracts.

## Difficult engineering decisions

- PostgreSQL job queues instead of a second broker for P0 operational
  simplicity and transactional durability.
- Immutable events plus replayable projections so read models can be rebuilt.
- Exact signed-byte fingerprinting without retaining raw signed bytes.
- Explicit Confirmed/Probable/Unknown certainty to avoid turning missing RPC
  evidence into a false failure.
- Solana Kit-first adapter with a neutral SDK boundary for future clients.

## Evidence in the repository

- Architecture: [overview](../architecture/architecture-overview.md) and
  [system design](../architecture/system-design.md).
- Reproducible [five-minute demo](portfolio-demo-script.md).
- React dashboard served by the Rust server, with token-gated live Overview,
  Traces, Trace detail, and Comparison API queries.
- Named security, resilience, and performance checks under [security](../security/)
  and [benchmarks](../benchmarks/).
- Docker image, Compose profiles, backup/restore scripts, and release checksum.

## Technology

Rust 1.98, Axum, SQLx, PostgreSQL 18, TypeScript, React, Vite, Solana Kit,
Docker Compose, and GitHub Actions.

## Pilot success measures

A real customer pilot should measure time-to-diagnosis, unknown-evidence rate,
landing/execution deltas by route and version, SDK overhead, observer coverage,
report generation time, retention cost, and restore time. These measurements
must use customer-approved privacy policy and staging data before production
claims are made.

## Scope disclaimer

This repository is a portfolio-quality P0 reference implementation. It does
not hold keys, submit transactions, provide hosted multi-tenant isolation, or
claim production availability. A pilot must complete runtime wiring,
authentication deployment, monitoring, and load/restore validation.
