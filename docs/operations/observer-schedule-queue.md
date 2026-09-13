# Observation worker

Landfall runs observation separately from the HTTP server:

```sh
bash scripts/compose.sh up -d server observer-worker postgres
```

The `observer-worker` service claims durable `observe_trace` rows using
PostgreSQL `FOR UPDATE SKIP LOCKED`. Set `LANDFALL_OBSERVER_CONCURRENCY` to
control how many jobs a worker process may claim concurrently; its default is
four. Multiple worker containers are also safe because a lease gives each job
one owner.

Each failed attempt is returned to the queue with exponential backoff: 5, 10,
20 seconds and so on, capped at five minutes. After attempt 10, the row moves
to `dead_letter` rather than being retried indefinitely. Inspect dead letters
through `GET /v1/system/health`, which reports `dead_letter_jobs`; retry and
dead-letter transitions are also written as structured worker logs with the
job ID and attempt number.

On `SIGTERM`, the worker stops claiming new jobs, waits for active polling
loops to stop, and leaves any uncompleted lease for recovery. On startup and
every 30 seconds it returns expired leases to `ready`, so a stopped container
does not lose a trace.
