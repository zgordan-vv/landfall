# Database size benchmark

Measure the fixture/database footprint after loading a known workload, then
call `storage_benchmark::storage_size(raw_bytes, events, traces)`. The result
reports bytes/event and bytes/trace and returns no estimate when either
denominator is zero. Record PostgreSQL version, schema/migration revision,
dataset size, and whether indexes/toast are included; otherwise numbers are not
comparable between runs.

This helper normalizes a measured footprint. It does not invent PostgreSQL
overhead and therefore cannot replace `pg_total_relation_size` measurements in
the deployment benchmark.
