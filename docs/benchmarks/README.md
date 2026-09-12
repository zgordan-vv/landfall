# Benchmark methodology and results

Benchmarks in this directory normalize measurements rather than claim universal
performance. Record commit SHA, toolchain/image versions, hardware, dataset
size, concurrency, warm-up policy, and whether the run used fixtures or a real
database/RPC endpoint.

Use the task-specific helpers for SDK overhead, compression, collector and
projection throughput, query p95, report duration, and retention rate. Report
median, p95, and maximum where applicable; never compare a dry-run retention
measurement with a destructive drop SLA.

Most helpers are deterministic unit-test normalizers. They do not replace a
staging load test, query-plan review, or provider-specific RPC test. See each
benchmark document for its command and limitations.
