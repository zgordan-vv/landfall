# Benchmark workload generator

`workload::generate_traces(count, seed)` creates repeatable synthetic traces.
The seed is the only input to outcome, event-count, route, and app-version
selection, so a benchmark can be rerun byte-for-byte with the same dataset.
The generator supports the Phase 15 design sizes (1,000, 10,000, and 50,000
traces); `burst_event_count(500)` defines the one-second burst target.

The corpus intentionally mixes success, expiry, execution-error, route, and
application-version dimensions. It is synthetic and must not be used to claim
production latency or storage behavior until the benchmark runners measure it.
