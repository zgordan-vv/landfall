# Projection metrics

`ProjectionMetrics` tracks successful projections, failures, dead-lettered
jobs, and current lag in milliseconds using atomic counters. The snapshot is
backend-neutral and can later be exported through Prometheus/OpenTelemetry
without changing worker code.
