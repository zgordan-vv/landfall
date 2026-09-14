# Health checks and graceful shutdown

Landfall exposes two HTTP health contracts:

- `/health/live` answers whether the process is alive;
- `/health/ready` reports dependency/read-model readiness and may be degraded.

The image also has a Docker `HEALTHCHECK` that runs `kill -0 1`. Debian slim
does not include a network client, so this probe intentionally checks only
process liveness. Kubernetes, Compose-side tooling, or a reverse proxy should
perform the HTTP readiness probe against `/health/ready`.

`STOPSIGNAL SIGTERM` gives the server a consistent termination signal. The
`WorkerSupervisor` cancellation token propagates that signal to background
workers, which stop accepting new work and join before exit. Keep a termination
grace period long enough for an in-flight database transaction to finish.

Validate the image metadata with:

```bash
docker inspect --format '{{json .Config.Healthcheck}}' landfall-server:local
docker inspect --format '{{.Config.StopSignal}}' landfall-server:local
```

The production container runs the HTTP server and workers. Verify the complete
runtime path with an HTTP readiness probe against `/health/ready`.
