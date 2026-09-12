# Trace-detail query benchmark

Collect trace-detail query durations in microseconds for a fixed workload and
pass them to `query_benchmark::summarize_query_latency`. The result reports
sample count, p50, p95, and maximum latency after deterministic sorting. An
empty run returns no result and cannot be reported as zero latency.

Record database version, indexes, page size, filter, warm/cold cache, and
concurrency with every measurement. The helper computes percentiles; it does
not execute SQL or conceal query-plan regressions.
