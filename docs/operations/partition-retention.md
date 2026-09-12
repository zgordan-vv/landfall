# Raw-event partition and retention primitives

`landfall-storage` exposes `ensure_raw_event_partition` for the daily range
`[day, day + 1 day)` and explicit drop helpers for retention jobs. Partition
identifiers are generated from a validated `time::Date` as `raw_events_YYYYMMDD`;
callers never supply SQL identifiers. Retention discovers only children of
`telemetry.raw_events` and removes partitions strictly older than the cutoff.

Creation should run ahead of ingestion for the next UTC day. Retention should
run after the configured data-retention window and must target a disposable or
backed-up dataset according to the deployment policy.
