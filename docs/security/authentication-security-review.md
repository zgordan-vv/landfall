# Authentication security review

## Verification

`cargo test -p landfall-server auth` covers missing, malformed, whitespace,
unknown, revoked, expired, and insufficient-scope credentials. Route-scope
tests cover ingest, traces, diagnostics, and admin classification.

Bearer tokens are accepted only in exact `Bearer <token>` form. The server
stores SHA-256 digests and compares fixed-size digests using a constant-time
XOR accumulator; plaintext tokens are never persisted or logged. Internal
failure reasons remain distinct so handlers can return generic public errors.

Timing behavior is reviewed at the primitive level; statistically measured
remote timing resistance is outside this unit-test document.

Tenant isolation is enforced in SQL, not only in UI state: each authenticated
token carries a `project_id`, which is bound into trace, overview, comparison,
diagnostic, recommendation, and system-status queries. Ingestion independently
checks each event’s `project_id` before it can reach durable storage.
