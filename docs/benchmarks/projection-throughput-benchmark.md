# Projection throughput and lag benchmark

After replaying a deterministic workload, record processed event count, elapsed
milliseconds, target watermark, and current watermark. Pass these values to
`projection_benchmark::measure_projection` to obtain events/sec and a
non-negative watermark lag. Record worker count, batch size, database version,
and whether the run started from a cold or warm cache. Zero elapsed time or an
empty workload returns no result instead of an infinite throughput claim.
