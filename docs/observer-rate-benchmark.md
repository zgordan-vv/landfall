# Observer batch/RPC rate benchmark

Record batches processed, RPC calls, rate-limited calls, and elapsed
milliseconds for a fixed observer window. `observer_benchmark::measure_observer`
returns calls/sec, calls per batch, and rate-limited ratio. It rejects empty
windows and impossible counters (`rate_limited > rpc_calls`).

The result must be recorded with route count, batch size, commitment level, and
provider mock latency. This helper normalizes counters; it does not itself make
network requests or claim provider capacity.
