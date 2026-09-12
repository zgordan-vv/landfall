# Landfall API reference (P0)

Base URL is the private `landfall-server` listener. The runtime currently
registers these routes:

| Method | Path | Purpose | Success |
|---|---|---|---|
| `POST` | `/v1/ingest` | Authenticate, validate, deduplicate, and durably accept an event batch | `202` new work, `200` fully duplicate batch |
| `GET` | `/health/live` | Process liveness | `200` |
| `GET` | `/health/ready` | Dependency/read-model readiness | `200` or degraded status |
| `GET` | `/openapi.json` | Generated OpenAPI document | `200` |
| `POST` | `/health/event` | Explicit synthetic health event (not persisted) | `200` |

## Ingestion contract

Send a versioned batch with an `Authorization: Bearer ...` header. The server
validates the envelope and event schema before committing event, deduplication,
and projection-job rows in one transaction. A successful `202` means the batch
was durably accepted, not that the transaction landed on-chain. Retrying the
same batch is safe; a fully duplicate batch returns `200`.

```bash
curl -X POST "$LANDFALL_URL/v1/ingest" \
  -H 'Authorization: Bearer <token>' \
  -H 'Content-Type: application/json' \
  --data-binary @batch.json
```

`400` indicates an invalid request or schema; `413` indicates body or
decompressed-size limits. Authentication is checked before expensive parsing.
The API never accepts private keys, seed phrases, raw signed bytes, or tokens
inside JSON fields.

## Contract discovery

Use `GET /openapi.json` for generated request/response schemas. Preserve decimal
strings exactly, scope identifiers to project/environment, and treat unknown
data-quality states as first-class values. Pagination, ETags, trace queries,
reports, and cohort endpoints are represented by read-model modules and become
public only when registered in the runtime router.
