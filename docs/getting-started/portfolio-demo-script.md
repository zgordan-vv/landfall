# Portfolio demo script

This is a short, deterministic walkthrough for an Upwork review. It focuses on
the evidence model and user-visible decisions rather than claiming live chain
coverage.

1. Start PostgreSQL with `just db-up` and build the server image with
   `just container-build`.
2. Start the dashboard with `pnpm --filter @landfall/dashboard dev`.
3. Open **Overview** and point out landing rate, execution success, unknown
   data, and the observer warning.
4. Open **Traces** and search for `tr_01HZX9`; open its **Trace detail** page.
5. Show the lifecycle evidence, attempt, finalized watermark, diagnosis, and
   missing simulation evidence.
6. Open **Comparison** to show descriptive cohort change and the small-sample
   warning.
7. Explain that ingestion is asynchronous and fail-open, while durable event
   acknowledgement requires the PostgreSQL transaction to commit.
8. Show the sanitized report, security runbooks, and benchmark methodology in
   the repository.

The current dashboard is fixture-backed and the server entry point remains a
composition placeholder. A customer pilot would replace fixtures with a
configured collector, observer RPC endpoints, migrations, and authenticated
deployment; the demo must state that boundary explicitly.
