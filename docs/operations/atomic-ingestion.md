# Atomic ingestion transaction

`ingest_atomically` writes the ingest batch, claims event identities with
`ON CONFLICT DO NOTHING`, persists only newly claimed raw events, and enqueues
one deduplicated `project_trace` job per trace. Any constraint or database
failure rolls back the complete batch, so an HTTP success can only be returned
after all durable work is committed.
