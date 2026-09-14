# Troubleshooting

## PostgreSQL will not become healthy

Run `just db-up`, then inspect `docker compose logs postgres`. Confirm the
credentials in `.env` and validate with `docker compose ps`. Do not publish the
database port to the host.

## Dashboard is blank or a route returns to Overview

Use hash routes such as `#traces` or `#traces/<trace-id>`. For the packaged
dashboard, rebuild the server image and restart the `server` Compose service;
it serves the React bundle itself. For development, run the Vite server after
building the dashboard. Enter a project token with `traces:read` and
`diagnostics:read` on first visit. The workspace reads the live API and shows
an empty state until the project has ingested traces.

## A trace is Unknown

Inspect the evidence checklist and observer route health. Unknown means required
evidence was not retained or was conflicting; it is not an inferred failure.
Check [data-quality grading](data-quality-grading.md) before changing rules.

## Docker build fails on dependencies

Use the pinned Rust/Node toolchain and run `docker build --check .`. Re-run with
`--progress=plain` to identify the failing stage. Do not bypass `--locked` or
`--frozen-lockfile` in a release build.
