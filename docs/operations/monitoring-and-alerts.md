# Monitoring and alerts

Landfall includes a Prometheus profile that scrapes the server from the private
Compose `data` network every 15 seconds. It evaluates the bundled alert rules
every 15 seconds and retains local time series for 30 days. Prometheus is
published only on `127.0.0.1:9090`; it is not a replacement for an authenticated
public observability endpoint.

## Start and validate monitoring

Start the application and monitoring profile with the same Compose environment
file used for the deployment:

```bash
bash scripts/compose.sh --profile monitoring up -d
bash scripts/check-monitoring-config.sh
```

Open `http://127.0.0.1:9090/targets`. The `landfall-server` target must be
`UP`. Open `http://127.0.0.1:9090/alerts` to see pending and firing rules.
The application endpoint itself remains `http://server:8080/metrics` inside the
private Docker network.

Before rolling out a rule change, validate the files without altering running
containers:

```bash
bash scripts/check-monitoring-config.sh
```

Prometheus reloads rules after a configuration reload. In a Compose deployment:

```bash
curl --fail --request POST http://127.0.0.1:9090/-/reload
```

The `--web.enable-lifecycle` flag exists solely for this localhost management
operation. Do not publish port 9090 beyond the host without putting a protected
reverse proxy in front of it.

## Notification delivery

The repository deliberately ships detection rules but does not invent a Slack,
email, PagerDuty, or webhook destination. Those endpoints and credentials are
deployment-owned secrets. A rule shown as **Firing** in Prometheus is evidence
that the condition is detected; it becomes a page only after Alertmanager is
configured.

Connect this Prometheus instance to the organization's Alertmanager using an
internal DNS name, then create routes by the `severity` label:

- `critical`: page the on-call operator immediately;
- `warning`: create a ticket or send a business-hours notification.

Test notification delivery with a controlled temporary alert and confirm both
receipt and resolution. Never put an Alertmanager webhook URL, token, or
recipient address into this repository.

## Alert catalogue and first response

### LandfallDatabaseUnavailable

The server could not obtain its aggregate PostgreSQL snapshot for two minutes.
Check PostgreSQL container health, the server's structured logs, connection
limits, disk space, and the database URL secret. Do not restart the database
or delete volumes as a first response. The alert resolves after a successful
scrape returns `landfall_database_up 1`.

### LandfallProcessNotReady

The HTTP process is alive but does not report readiness for two minutes. Query
`/health/ready`, inspect the startup error in structured logs, and confirm that
migrations and PostgreSQL are reachable. Route traffic away from this instance
until readiness recovers.

### LandfallDeadLetterJobs

One or more durable jobs exhausted their retry budget for five minutes. Inspect
the affected jobs through the system health and worker runbooks, identify the
underlying RPC or projection failure, and replay only after the cause is fixed.
Do not blindly requeue every dead letter: a deterministic bad payload will fail
again.

### LandfallReadyQueueBacklog

At least 100 jobs have waited in `ready` state for 15 minutes. Check whether
worker replicas are alive, whether the RPC provider is throttling, and whether
database latency has increased. Increase worker capacity only after confirming
that the downstream provider and database can sustain it.

### LandfallExpectedIngestionSilent

At least one RPC route is enabled, yet no event has been received in the prior
24 hours. This rule is intentionally a warning: an idle pilot project can be
legitimate. Keep it enabled only for deployments expected to receive regular
traffic; otherwise remove it from `deploy/monitoring/alerts.yml` or silence it
with a documented expiration. For an expected active route, check route status,
provider reachability, and application ingest clients.

### LandfallX402PaymentFailures

One or more payment attempts reached the durable `failed` terminal state in the
last 24 hours. Open the project's **x402 payments** audit view and compare the
stable `reason_code` with the merchant response and facilitator logs. The
payment signature and wallet material are deliberately unavailable in Landfall.
Do not retry a charge from the alert alone: first determine whether the merchant
served the resource and whether the failure was recorded after an ambiguous
network timeout.

### LandfallX402PaymentsPending

Ten or more authorizations remain in `approved` state for 15 minutes. This can
mean callers are not reporting terminal outcomes, a merchant is unavailable, or
the client was interrupted after policy approval. Inspect recent audit records
and client logs; safely settle only outcomes that the client can verify. Pending
records are not evidence that payment occurred.

## Ownership and review

Review alert thresholds after the first two weeks of real traffic. The bundled
values are conservative operational defaults, not an SLO. Record threshold
changes, the expected traffic assumption, and the on-call owner in the
deployment's change record.

## Scheduled local backup

For a single-host pilot, use `scripts/scheduled-backup-postgres.sh` from a
system timer or cron job. It creates a checksummed archive, verifies it before
returning, skips an overlapping run, refuses to run below a free-space floor,
and prunes only prior Landfall archives after the configured retention period.

```bash
LANDFALL_ENV_FILE=/root/landfall/landfall.env \
LANDFALL_BACKUP_DIR=/root/landfall/backups \
LANDFALL_BACKUP_RETENTION_DAYS=7 \
./scripts/scheduled-backup-postgres.sh
```

This is availability protection, not disaster recovery: archives on the same
host do not survive loss of that host. Copy each verified archive and manifest
to encrypted off-host storage before making an availability promise.
