# Metric Definitions

Metric semantics are versioned as `metric-definitions-v1` and implemented as
pure functions over a validated `TraceProjection`.

- `terminal_eligible`: observation completeness is `complete`.
- `landed`: landing is processed, confirmed, or finalized.
- `execution_success` / `execution_failure`: the corresponding on-chain state.
- `application_success`: the application reported a successful outcome.

Incomplete, conflicting, or in-progress traces are excluded from terminal
denominators. Landing, execution, and application flags remain independent so
an aggregate cannot silently turn an observation gap into a failure.
