# Solana lifecycle fixtures

[`fixtures/solana/lifecycle-scenarios.json`](../fixtures/solana/lifecycle-scenarios.json)
contains deterministic scenarios for the adapter and observer suites:

- normal confirmed success;
- simulation execution failure with no submission;
- client timeout followed by later on-chain confirmation;
- accepted submission that is not observed after `lastValidBlockHeight`.

Validate the complete set with `node scripts/check-solana-lifecycle-fixtures.mjs`.
