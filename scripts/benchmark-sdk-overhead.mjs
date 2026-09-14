import { performance } from "node:perf_hooks";
import { LandfallSdk } from "../packages/sdk-ts/dist/index.js";

const iterations = Number(process.env.LANDFALL_BENCH_ITERATIONS ?? 100_000);
const sdk = new LandfallSdk({ collectorUrl: "http://localhost:8080" });
const traceId = "0198ef10-0007-7000-8000-000000000201";
if (typeof globalThis.gc === "function") globalThis.gc();
const before = process.memoryUsage().heapUsed;
const started = performance.now();
for (let index = 0; index < iterations; index += 1) {
  sdk.startTrace(traceId).emit({ event_type: "benchmark.event", index });
}
const elapsedMs = performance.now() - started;
if (typeof globalThis.gc === "function") globalThis.gc();
const after = process.memoryUsage().heapUsed;
console.log(
  JSON.stringify({
    iterations,
    elapsed_ms: elapsedMs,
    ns_per_capture: (elapsedMs * 1e6) / iterations,
    heap_delta_bytes: after - before,
  }),
);
