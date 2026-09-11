# Migration runner and version health

`landfall-storage` embeds the append-only SQL files with `sqlx::migrate!`, so
the application binary and its schema set are versioned together. Startup or a
deployment command can call `run_migrations(&pool)`; SQLx records applied
versions and checksums in `_sqlx_migrations`.

`migration_health(&pool)` reads that ledger and compares it with the embedded
versions. The returned `MigrationHealth::is_current()` is false when a version
is missing, failed, or the database is ahead of the binary. This gives an
operational readiness check without exposing migration SQL through the API.
