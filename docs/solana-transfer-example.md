# Controlled SOL-transfer example

`examples/solana-kit-transfer.mjs` is a safe lifecycle harness for local
validator or Devnet work. It uses a tiny 1,000-lamport transfer description,
an injected client, and dry-run mode by default. No keypair or private key is
read. To opt into the submission branch, an operator must explicitly set:

```sh
SOLANA_CLUSTER=local ALLOW_SUBMISSION=true node examples/solana-kit-transfer.mjs
```

The current harness keeps the client injected so it is deterministic. The
future `@solana/kit` 8.2.0 adapter can replace those methods without changing
the lifecycle capture or safety gates.
