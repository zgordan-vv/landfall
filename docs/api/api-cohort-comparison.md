# Cohort comparison

`GET /v1/comparison` (and the read-only `GET /demo/v1/comparison`) answers one
question: _we changed how we send transactions (compute limit, RPC route, retry
policy, fees); did reliability actually improve?_ It compares a **baseline**
environment with a **candidate** environment of the same project. See
[ADR-009](../adr/009-explicit-comparison-cohorts.md) for why roles are explicit.

## Roles

Roles are never inferred from recency or trace counts. They come from, in order:

1. `baseline_environment_id` and `candidate_environment_id` query parameters
   (both or neither; `400 incomplete_cohort_roles` otherwise), or
2. the project's designated cohorts, set by a project administrator:

```sh
curl -sS -X PUT "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/comparison-cohorts" \
  -H "Authorization: Bearer $LANDFALL_PROJECT_ADMIN_TOKEN" -H 'Content-Type: application/json' \
  --data '{"baseline_environment_id":"…","candidate_environment_id":"…"}'
```

With neither, the response is `200` with `"status": "not_configured"`, no
metrics, and the project's environments so a client can say what to choose
from. Both environments must belong to the caller's project (`404
environment_not_found`) and differ (`400 identical_cohort_roles`).

Each environment may carry a one-line `description` (at most 280 characters)
saying what it changes, for example `tuned: compute limit from simulation +
headroom`. Set or clear it with
`PUT /v1/control/projects/$PROJECT_ID/environments/$ENVIRONMENT_ID/description`
and `{"description": "…"}` or `{"description": null}`. Descriptions are shown to
everyone who can read comparisons, including public demo visitors; never put
secrets or endpoints in them.

## Window

`window_hours` (1–2160, default 168) selects traces whose **first raw event**
falls in `[now - window_hours, now)`. The response states the window as
`window.start` and `window.end`. More than 100 000 traces in the window returns
`422 comparison_window_too_large`.

## Metrics (`cohort-comparison-v2`)

Each metric returns the baseline and candidate value, `delta` (candidate minus
baseline), `better_when`, an `outcome`, and its own `sample_size`.

| Key                      | Better | Value                                                                                                     |
| ------------------------ | ------ | --------------------------------------------------------------------------------------------------------- |
| `landing_rate`           | higher | Submitted traces observed at processed/confirmed/finalized ÷ traces with at least one submission          |
| `execution_success_rate` | higher | Traces whose on-chain execution succeeded ÷ traces with a known execution result                          |
| `expiry_rate`            | lower  | Submitted traces whose blockhash expired without observed inclusion ÷ traces with at least one submission |
| `retry_rate`             | lower  | Submitted traces with more than one attempt or a scheduled retry ÷ traces with at least one submission    |
| `median_confirmation_ms` | lower  | Median first submission → first confirmed/finalized status observation, over traces with both events      |
| `median_fee_lamports`    | none   | Median fee from `solana.execution.enriched`; fees can be raised on purpose, so it is not rated            |

`outcome` is `improved`, `regressed`, `unchanged`, `changed` (unrated metric),
or `no_evidence` when either cohort has nothing to measure. A metric without
evidence is `null`, never `0`.

Latency resolution is bounded by the observer's polling interval and by clock
differences between the application (submission) and the observer
(confirmation); a confirmation timestamped before the submission is dropped
rather than reported as a negative latency. For the same reason a latency
change smaller than 5% of the baseline median, or 50 ms, is reported as
`unchanged`.

## Sample size

`minimum_sample` is 30. `sufficient_sample` is `false` when either cohort has
fewer than 30 traces in the window, and each metric has its own
`sufficient_sample` for its denominator. At 30 traces a rate near 50% still has
a 95% interval of roughly ±18 percentage points, so the floor only prevents
reading noise as a result. The comparison is descriptive: no statistical
significance is claimed.

## Verification

```sh
cargo test -p landfall-server cohort_comparison
LANDFALL_TEST_DATABASE_URL=postgres://… cargo test -p landfall-server comparison_tests
```
