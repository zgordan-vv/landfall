# Trace deletion and tombstones

Deletion is represented by a durable job and an append-only `TraceTombstone`.
The ledger validates actor/time fields, records the reason for audit, and
rejects a second deletion for the same trace. Read paths can consult
`is_deleted` before returning any searchable or derived trace data; the
tombstone itself remains available for operational audit.
