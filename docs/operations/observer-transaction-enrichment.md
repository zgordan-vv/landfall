# `getTransaction` enrichment

`get_transaction()` fetches optional execution details for a signature and
passes `maxSupportedTransactionVersion: 0` to the RPC provider. This makes the
observer compatible with legacy and v0 responses while treating a `null` result
as not-yet-available evidence rather than an error. Detailed field
normalization is performed by the following enrichment step.
