# Repository boundaries

Services depend on `PersistenceRepository`, not directly on SQLx. The
`PgRepositories` adapter owns the `PgPool` and translates database errors to
the storage-layer `RepositoryError`. This keeps transaction orchestration and
future repository methods replaceable for tests or another persistence backend.
