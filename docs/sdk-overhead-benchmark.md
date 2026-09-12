# SDK overhead benchmark

Run `node --expose-gc scripts/benchmark-sdk-overhead.mjs` after building
`packages/sdk-ts`. The harness performs a fixed number of non-blocking
`startTrace().emit()` calls and prints JSON containing elapsed time,
nanoseconds per capture, and heap delta. Set `LANDFALL_BENCH_ITERATIONS` to
repeat at another scale.

This measures the current instrumentation facade in one Node process; it is
not a browser benchmark and must not be presented as a production SLA. Record
Node version, CPU, iteration count, and whether `--expose-gc` was used with
each result.
