# `getTransaction` enrichment

`get_transaction()` fetches optional execution details for a signature and
passes `maxSupportedTransactionVersion: 0` to the RPC provider. This makes the
observer compatible with legacy and v0 responses while treating a `null` result
as not-yet-available evidence rather than an error. Detailed field
normalization is performed by the following enrichment step.

When a status lookup finds a signature, the server worker performs this request
in the same observation job. A non-null response produces one immutable
`solana.execution.enriched` event alongside `solana.status.observed`, then
refreshes the trace projection. The event retains only slot, commitment,
success/failure, fee, compute units, block time, and whether logs exist; raw
transaction bytes and log contents are never stored.
