# Validity cases

The observer classifies a transaction as `RecentBlockhash`, `DurableNonce`, or
`Unsupported` before evaluating expiry. Only recent-blockhash transactions use
the strict block-height rule. Durable nonce returns an indeterminate result,
while unsupported versions return `UnsupportedValidity`; neither is reported
as a false expiration.
