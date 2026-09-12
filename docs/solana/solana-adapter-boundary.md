# Solana adapter boundary

`SolanaClientPort<TPrepared, TSigned, TSignature>` is the seam between the
neutral SDK and a concrete Solana client. It describes only capabilities needed
by the lifecycle (blockhash, simulation, application-owned signing, submission,
and confirmation) and uses generics for client-specific transaction types.

The boundary deliberately does not import `@solana/kit`, accept keypairs, or
own customer retry policy. `preserveCustomerOperation()` demonstrates the
failure-isolation contract: the wrapped operation's original value and error
identity are returned unchanged.

`captureLatestBlockhash()` normalizes the client result and preserves
`lastValidBlockHeight` as an exact decimal string, so large Solana heights are
never routed through lossy JavaScript numbers.
