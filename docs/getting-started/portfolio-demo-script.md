# Portfolio demo script

This is a short, deterministic walkthrough for an Upwork review. It focuses on
the evidence model and user-visible decisions rather than claiming live chain
coverage.

1. Start the Compose stack with `bash scripts/compose.sh up --build`.
2. Open the server URL (default `http://127.0.0.1:8080/`). The server serves
   the dashboard bundle; no separate frontend server is required.
3. Use **Onboarding** to create a project, dashboard token, environment, RPC
   route, and SDK ingestion token. Store every token before leaving the page.
4. Enter the dashboard token in the workspace access form. Open **Overview**
   to show metrics calculated from the live read model.
5. Open **Traces**, select an actual ingested trace, then show its lifecycle
   evidence, attempts, finalized watermark, diagnosis, and missing evidence.
6. Open **Comparison** to show descriptive cohort change and the small-sample
   warning for live trace cohorts.
7. Explain that ingestion is asynchronous and fail-open, while durable event
   acknowledgement requires the PostgreSQL transaction to commit.
8. Show the sanitized report, security runbooks, and benchmark methodology in
   the repository.

The dashboard is a live API client: it never supplies portfolio fixtures as
operational data. Its usefulness depends on a configured collector, a valid
project token, PostgreSQL, and an observer RPC route. The P0 deployment is
self-hosted and token-based; it is not yet a hosted multi-tenant SaaS or a
browser-session login product.
