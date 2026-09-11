# Test database lifecycle and seed helpers

Integration tests use the same SQLx migration runner as production, then call
`reset_test_data` to truncate application tables while preserving
`_sqlx_migrations`. `seed_control_plane` inserts a deterministic project and
environment in one transaction and returns their IDs. This makes tests
repeatable, prevents fixture leakage between cases, and exercises real foreign
keys instead of an in-memory substitute.

The helpers are intentionally explicit: a test must opt into cleanup and seed
data, and they should only be used against a disposable PostgreSQL database.
