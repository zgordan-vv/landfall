# Overview and comparison query benchmark

Collect durations separately for overview and comparison requests, then call
`dashboard_query_benchmark::summarize_dashboard_queries`. The response keeps
each class's sample count, p50, p95, and max; an empty class remains `null` and
cannot hide an unmeasured path.

Record filter cardinality, cohort size, page size, database/index revision,
cache state, and concurrency. Compare p95 values only between runs with the
same workload and environment.
