# Troubleshooting

## PostgreSQL will not become healthy

Run `just db-up`, then inspect `docker compose logs postgres`. Confirm the
credentials in `.env` and validate with `docker compose ps`. Do not publish the
database port to the host.

## Dashboard is blank or a route returns to Overview

Rebuild the dashboard with `pnpm --filter @landfall/dashboard build`, restart
the Vite dev server, and use hash routes such as `#traces` or
`#traces/tr_01HZX9`. The portfolio dashboard is fixture-backed.

## A trace is Unknown

Inspect the evidence checklist and observer route health. Unknown means required
evidence was not retained or was conflicting; it is not an inferred failure.
Check [data-quality grading](data-quality-grading.md) before changing rules.

## Docker build fails on dependencies

Use the pinned Rust/Node toolchain and run `docker build --check .`. Re-run with
`--progress=plain` to identify the failing stage. Do not bypass `--locked` or
`--frozen-lockfile` in a release build.
