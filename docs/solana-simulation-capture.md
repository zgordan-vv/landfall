# Solana simulation capture

`normalizeSimulationResult()` converts a client response into a neutral
snapshot: `succeeded`, `execution_error`, `blockhash_not_found`, or
`malformed_response`. `unitsConsumed` is serialized as an exact decimal string
(including `bigint` values), and only a boolean `logsPresent` signal is kept.
The helper does not serialize a transaction object or import a Solana client.
