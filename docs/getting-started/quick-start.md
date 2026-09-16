# Landfall quick start

This starts the real local API, PostgreSQL, and observation worker, then opens
the dashboard. No Solana private key is required. You may use the public Solana
HTTPS endpoint for a first route, or supply your own RPC endpoint.

## Prerequisites

- Docker with Compose v2;
- Rust 1.98.0, Node.js 24.20.0, pnpm 11.25.0, and just 1.58.0.

## Run

```bash
cp .env.example .env
LANDFALL_BOOTSTRAP_TOKEN=local-bootstrap-token bash scripts/compose.sh up -d --build
pnpm install --frozen-lockfile
pnpm --filter @landfall/dashboard dev
```

Open the local URL printed by Vite and choose **Get started**. Enter the
bootstrap token above, create a project, environment, HTTPS RPC route, and an
`ingest:write` SDK token. The Vite development server proxies `/v1` requests
to the real Landfall server on port 8080, so no browser CORS exception is
needed. Store each newly displayed token in a secret manager: it is shown once.

For the user-facing setup path, token types, empty states, and safe support
handoff, read the [customer onboarding and support guide](customer-onboarding-and-support.md).

## Stop and reset

Stop the database while preserving its volume:

```bash
bash scripts/compose.sh down
```

To explicitly remove the local database volume and start fresh:

```bash
bash scripts/compose.sh down --volumes
```

`db-reset` is destructive and is intentionally separate from normal shutdown.
For the full event protocol, API, and production-like deployment workflow, use
the links in the [README documentation map](../README.md#documentation-map).
