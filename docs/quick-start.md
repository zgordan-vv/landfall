# Landfall quick start

This starts the local PostgreSQL dependency, validates the server image, and
opens the fixture-backed dashboard. No Solana RPC account or private key is
required.

## Prerequisites

- Docker with Compose v2;
- Rust 1.98.0, Node.js 24.20.0, pnpm 11.25.0, and just 1.58.0.

## Run

```bash
cp .env.example .env
just db-up
just container-build
pnpm install --frozen-lockfile
pnpm --filter @landfall/dashboard dev
```

Open the local URL printed by Vite. Navigate through Overview, Traces, Trace
detail, and Comparison. The fixture data makes this walkthrough deterministic.

## Stop and reset

Stop the database while preserving its volume:

```bash
just db-down
```

To explicitly remove the local database volume and start fresh:

```bash
just db-reset
```

`db-reset` is destructive and is intentionally separate from normal shutdown.
For the full event protocol, API, and production-like deployment workflow, use
the links in the [README documentation map](../README.md#documentation-map).
