# Disposable health-check event

`POST /health/event` returns a generated, synthetic `solana.trace.created`
event with a UUID v7. The response is explicitly marked `synthetic` and states
that it is not persisted, so operators can verify routing, request IDs, and
JSON serialization without creating customer telemetry or credentials.
