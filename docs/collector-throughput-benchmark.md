# Collector throughput benchmark

`collector_benchmark::measure_collector` parses a fixed event payload repeatedly
and reports elapsed time, events/sec, and p95 parse latency. Run its test with
`cargo test -p landfall-server collector_benchmark` or call it from a benchmark
runner using the deterministic workload generator.

This isolates collector CPU work and malformed-payload handling. It does not
measure TCP/TLS, PostgreSQL commit latency, queue contention, or compression;
those boundaries require an end-to-end runner before making production capacity
claims.
