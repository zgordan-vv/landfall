# Solana Kit integration guide

Landfall's Kit adapter wraps an application-owned transaction lifecycle. It
records evidence around the calls your application already makes; it never
owns keys, signs for the application, submits on its behalf, or changes retry
policy.

## Recommended integration order

1. Create a Landfall context and keep `trace_id` with the business action.
2. Wrap `getLatestBlockhash()` so the blockhash and exact validity height are
   recorded.
3. Wrap simulation and normalize `unitsConsumed`/logs presence.
4. Wrap the application's signer; only the exact serialized signed bytes are
   fingerprinted transiently in memory.
5. Wrap every submission attempt with route and preflight configuration.
6. Wrap confirmation waiting and send sanitized events asynchronously.

The runnable dry-run harness is
[`examples/solana-kit-transfer.mjs`](../../examples/solana-kit-transfer.mjs).
It uses an injected fake client and cannot submit funds. Keep
`ALLOW_SUBMISSION` unset while developing; enabling real devnet submission
requires an explicit operator decision and a separately configured signer.

P0 supports `@solana/kit` 8.2.0 on Node 24. The compatibility lane is frozen in
[`config/solana-kit-lane.json`](../../config/solana-kit-lane.json) and the
[support matrix](../architecture/support-matrix.md).
