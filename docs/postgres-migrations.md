# PostgreSQL migration foundation

Migration `0001_create_schemas.sql` creates the four ownership boundaries:
`control`, `telemetry`, `work`, and `reporting`. It also creates minimal
metadata, ingest-batch, job-type, and report-run tables used by later phases.

The migration is append-only and safe to replay with `IF NOT EXISTS` and
idempotent seed insertion. It was smoke-tested against the Docker Compose
PostgreSQL instance.
