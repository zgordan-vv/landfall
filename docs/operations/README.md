# Operations runbooks

Use these runbooks in this order when operating a local or pilot deployment:

1. [Quick start](../getting-started/quick-start.md) — provision the local
   environment.
2. [Local ingestion smoke test](local-ingestion-smoke.md) — seed control-plane
   identifiers and verify durable `202`/duplicate behavior.
3. [PostgreSQL migrations](postgres-migrations.md) and [migration health](migration-health.md)
   — apply and verify schema changes.
4. [Health and shutdown](health-and-shutdown.md) — distinguish liveness,
   readiness, and graceful worker termination.
5. [Structured logging](structured-logging.md) — correlate JSON operational
   events by request ID without exposing telemetry or credentials.
6. [Prometheus metrics](metrics.md) — scrape aggregate readiness, job, and
   ingestion gauges from the private network.
7. [Monitoring and alerts](monitoring-and-alerts.md) — start Prometheus,
   configure notifications, and respond to operational alerts.
8. [Backup and restore](backup-restore-runbook.md) — create checksummed dumps
   and perform a guarded restore.
9. [Retention worker](retention-worker.md) and [partition retention](partition-retention.md)
   — preview and execute bounded cleanup.
10. [Project/trace workers](project-trace-worker.md) and [reproject command](reproject-command.md)
   — inspect or replay durable jobs.
11. [Report jobs](report-jobs.md) and [report artifacts](report-artifact-storage.md)
   — generate and safely serve exports.
12. [Golden incident runbook](golden-incident-runbook.md) — reproduce known
   lifecycle failures and compare evidence.

For security-specific response use the [security section](../security/). The
current server entry point is a composition placeholder; pilot deployment must
complete runtime wiring, migrations, secrets, and monitoring before production
traffic is allowed.
