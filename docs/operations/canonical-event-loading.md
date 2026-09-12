# Canonical event loading

Projection reads raw telemetry through `load_events_for_trace`. The query
requires project, environment, trace, and a half-open `[from, until)` time
range. Results use a stable `(occurred_at, received_at, event_id)` ordering,
which makes replay deterministic and prevents cross-tenant or unbounded scans.
