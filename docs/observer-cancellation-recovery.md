# Observer cancellation and restart recovery

`ObserverCancellation` wraps Tokio's `CancellationToken` so workers can stop
cooperatively between RPC operations and await cancellation without polling a
shared flag. `rehydrate_queue()` loads ready durable schedules into a new
`ObservationQueue` after restart; process-local queue state is never treated as
the source of truth.
