# Recommendation disposition and history

The recommendation API records `accepted`, `rejected`, `implemented`, or
`not_applicable` as append-only `DispositionRecord` entries. A later decision
does not overwrite an earlier one; `latest_disposition` is only a convenience
view over the immutable history. Each entry carries an actor, timestamp, and
optional bounded reason for auditability. Duplicate disposition IDs are
rejected to make retries idempotent.

## Verification

Run `cargo test -p landfall-server recommendation_disposition`. Decisions append
history; duplicate disposition IDs are rejected.
