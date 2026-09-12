# Local ingestion smoke test

This verifies the first durable vertical slice without real Solana traffic.
The Compose server waits for PostgreSQL health and runs migrations before
opening port 8080.

```bash
bash scripts/compose.sh up -d --wait server postgres
bash scripts/seed-demo-control-plane.sh
```

In another terminal, post a batch whose UUIDs match the seeded control plane:

```bash
curl -i -X POST http://127.0.0.1:8080/v1/ingest \
  -H 'content-type: application/json' \
  --data '{"events":[{"schema_version":"1.0","event_type":"solana.trace.created","event_id":"0198ef00-0000-7000-8000-000000000401","project_id":"0198ef00-0000-7000-8000-000000000100","environment_id":"0198ef00-0000-7000-8000-000000000200","trace_id":"0198ef00-0000-7000-8000-000000000300","occurred_at":"2026-09-12T12:00:00Z","attributes":{"flow":"checkout","transaction_version":"legacy"}}]}'
```

Expected response is `202` with `accepted: 1`. Repeating the request should
return `200` with `duplicate: 1`. A production smoke test must additionally
query `telemetry.raw_events` and `work.jobs`; this fixture does not authorize
public traffic or represent a production tenant.
