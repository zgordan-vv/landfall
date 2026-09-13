# API authentication and tenant isolation

All `/v1/*` routes require `Authorization: Bearer <token>` when durable
PostgreSQL storage is configured. Health checks and `/openapi.json` remain
unauthenticated so an orchestrator can inspect service availability.

The server stores only a SHA-256 digest in `control.api_tokens`, never the
plaintext token. Token comparison is constant-time; revoked and expired tokens
are rejected. Public responses do not disclose whether a token is unknown,
revoked, or expired: each receives `401 unauthorized`. A valid token without
the required permission receives `403 insufficient_scope`.

| Route family | Required scope |
| --- | --- |
| `POST /v1/ingest` | `ingest:write` |
| traces, overview, comparison | `traces:read` |
| diagnostics and recommendations | `diagnostics:read` |
| `/v1/system/status` | `admin` |

Every token belongs to exactly one `project_id`. Read queries add that project
filter and ingestion rejects events for another project with `403
project_forbidden`; presenting a trace ID from another tenant therefore returns
the same `404 trace_not_found` response as a missing trace.

To create a local token, generate a random plaintext value outside the shell
history, calculate its digest, and insert only the digest. Give the plaintext
to the client once, then discard it:

```sh
TOKEN="$(openssl rand -hex 32)"
DIGEST="$(printf %s "$TOKEN" | shasum -a 256 | awk '{print $1}')"
printf 'copy this once: %s\n' "$TOKEN"
```

Use `decode($DIGEST, 'hex')` for `token_hash` in the project’s
`control.api_tokens` record. Rotation creates a replacement record before
setting `revoked_at` on the old record.
