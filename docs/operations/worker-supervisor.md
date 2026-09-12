# Worker supervisor

`WorkerSupervisor` owns spawned background tasks and gives each worker a child
`CancellationToken`. Shutdown cancels the shared token, then awaits every
tracked handle, providing deterministic graceful termination for projection,
observation, and retention workers. Worker-specific loops remain separate from
the supervisor and decide how frequently to check cancellation.
