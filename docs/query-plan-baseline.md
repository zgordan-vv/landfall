# Query-plan baseline

`scripts/check-postgres-query-plans.sh` creates a disposable UTC-day partition,
runs bounded trace and raw-event lookups with sequential scans disabled, and
asserts that the intended indexes are selected:

- `reporting.traces` → `traces_environment_updated_idx`;
- `telemetry.raw_events` → `raw_events_trace_time_idx`.

The temporary partition is dropped by a shell trap. This is a smoke baseline,
not a production benchmark: real cardinality and statistics must be measured
again after representative data loads.
