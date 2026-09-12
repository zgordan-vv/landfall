# Report jobs

`ReportJobRegistry` and `run_report_worker` provide the asynchronous report
job foundation. Creation records a queued job and its frozen projection
watermark. The bounded queue applies backpressure; the worker transitions jobs
through `queued → running → completed` or `failed`, preserving an error for
status queries. Cancellation stops intake without inventing a successful
result.

## Verification

Run `cargo test -p landfall-server report_jobs`. The worker test observes the
queued-to-running-to-completed transition.
