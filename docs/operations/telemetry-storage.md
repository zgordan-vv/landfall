# Telemetry storage

Migration `0003_create_telemetry_events.sql` separates idempotency from raw
payload storage:

- `telemetry.event_dedup` records the first accepted `event_id` and payload
  hash;
- `telemetry.raw_events` is a range-partitioned parent keyed by `occurred_at`.

Indexes are limited to environment/time and trace/time access paths. Payload
JSON remains unindexed so query cost is predictable; typed projections will be
stored separately in later migrations.
