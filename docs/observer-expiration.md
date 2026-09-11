# Expiration evaluation

`evaluate_expiration()` marks a recent-blockhash transaction expired only when
`current_block_height` is strictly greater than `last_valid_block_height`.
Equality remains valid at the boundary. Durable-nonce transactions return
`IndeterminateDurableNonce` because recent-blockhash expiration rules do not
apply to them.
