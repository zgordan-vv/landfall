# ADR-009: Explicit comparison cohorts

- **Status:** Accepted
- **Date:** 2026-10-21
- **Decision owners:** Landfall maintainers
- **Related requirements:** Environment comparison (Comparison view)
- **Related ADRs:** [ADR-002](002-immutable-event-inputs-and-relational-projections.md), [ADR-008](008-code-first-openapi-with-utoipa.md)
- **Supersedes:** None
- **Superseded by:** None

## Context

`GET /v1/comparison` chose the two most recently updated environments and called
the older one "baseline". Roles flipped whenever the other environment received
a trace, so the public demo at one point labelled the improved environment as
the baseline and showed a landing-rate _drop_. The response also exposed only
raw environment UUIDs and a single landing-rate pair, and the dashboard colored
any change as good. A comparison whose direction depends on write timing cannot
answer "did our change improve reliability?".

## Decision drivers

- Roles must be stable and chosen by a person who knows what changed.
- The page must say what is being compared, in words.
- No fabricated values: missing evidence is reported as missing.
- Keep the modular monolith: no new services, a single small migration.

## Options considered

### Option A — Keep inferring roles, but by creation time or trace count

Still a guess. Creation order says nothing about which environment carries the
change, and a heuristic that is right today flips when another environment is
added.

### Option B — Query parameters only

Stable per request, but the dashboard and the public demo would need a role
picker or hard-coded identifiers, and two viewers could see opposite readings.

### Option C — Persisted project setting, overridable by query parameters

A project administrator designates both roles once
(`control.comparison_cohorts`); API callers may pass explicit roles for ad-hoc
comparisons. Without either, the API reports "not configured".

## Decision

Option C. `control.comparison_cohorts` holds one row per project with
`baseline_environment_id` and `candidate_environment_id`; composite foreign
keys to `control.environments (project_id, environment_id)` guarantee both
belong to the project, and a check forbids identical roles.
`control.environments.description` (optional, at most 280 characters) states
what each environment changes.

`GET /v1/comparison` and `GET /demo/v1/comparison` return a new
`ComparisonReport` shape (status, role source, window, cohort names and
descriptions, six metrics with direction and per-metric sample size). This
replaces the previous `ComparisonSummary` in place under `/v1`: the endpoint was
not in the published OpenAPI document, the API is pre-1.0 (`0.1.0`), and its
only consumer is the dashboard shipped in the same server image. A third-party
client of the old shape must move to the new fields; no `/v2` route is added.

## Detailed design and boundaries

- Admin endpoints: `GET`/`PUT /v1/control/projects/{project_id}/comparison-cohorts`
  and `PUT /v1/control/projects/{project_id}/environments/{environment_id}/description`
  (scope `project:admin`).
- Metric computation is a pure function over per-trace evidence in
  `landfall-server::cohort_comparison`; storage reads projected trace states and
  the trace's raw events (submission, retry, status, execution-enriched).
- This ADR does not add statistical significance testing, per-route or per-flow
  cohorts, or time-sliced (before/after) cohorts within one environment.

## Consequences

### Positive

- Readings no longer flip with write timing; the demo says what is staged.
- Direction-aware metrics stop presenting regressions as improvements.

### Negative

- A project must be configured before Comparison shows data.
- The `/v1/comparison` response shape changed without a version bump.

### Risks and mitigations

| Risk                                    | Impact                                  | Mitigation or detection                                                    |
| --------------------------------------- | --------------------------------------- | -------------------------------------------------------------------------- |
| Small cohorts read as conclusions       | Wrong decision about a sending change   | `minimum_sample` (30) per cohort and per metric; dashboard notice           |
| Descriptions leak sensitive details     | Exposure on the public demo             | Documented as public; one line, no control characters, 280-character limit |
| Large windows scan many raw events      | Slow comparison queries                 | 90-day window cap, 100 000-trace cap, trace and raw-event indexes          |

## Security and privacy impact

The new admin endpoints use the existing `project:admin` authorization and
project scoping. Descriptions are operator-supplied text visible to comparison
readers, including the public demo; they are documented as public. No secrets,
endpoints, or identifiers beyond environment UUIDs are added to responses.

## Reliability and failure behavior

Storage errors return `503`; misconfigured roles return `400`/`404`; an
unconfigured project returns `200 not_configured`. The comparison never falls
back to a guessed pair.

## Performance and capacity impact

One lateral aggregate over each windowed trace's raw events, using
`traces_environment_updated_idx` and `raw_events_trace_time_idx`. Reconsider if
p95 comparison latency exceeds the dashboard query budget.

## Operational impact

Migration `0017_create_comparison_cohorts.sql` adds a column, a unique
constraint, and a table; it is additive. After deploying, an administrator sets
the cohorts and descriptions (see `scripts/demo-traffic/README.md` for the
public demo).

## Verification

- `cargo test -p landfall-server cohort_comparison` (role selection, metrics).
- `comparison_tests` against PostgreSQL: roles stay fixed when the other
  environment is updated more recently, windowing, cross-project rejection.

## Rollout and migration

Deploy, then `PUT` the cohorts. Until then the page shows "not configured".
Rolling back the binary leaves an unused table and column.

## Reconsideration triggers

- Customers need before/after cohorts inside one environment.
- Comparison queries exceed the dashboard latency budget.

## References

- [Cohort comparison API](../api/api-cohort-comparison.md)
