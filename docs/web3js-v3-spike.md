# `@solana/web3.js` v3 compatibility spike

The project keeps `@solana/kit` 8.2.0 as the only P0 Solana client. A separate
time-boxed spike may evaluate `@solana/web3.js` v3 for future demand, but it is
not a production adapter and must not widen the support claim.

The spike has an eight-hour budget and must collect evidence for installation,
typed compilation, blockhash capture, simulation, submission, confirmation,
and privacy behavior. If the evidence is incomplete at the deadline, the
spike stops with the client remaining `recognized, unsupported`.

Validate the guardrails with `node scripts/check-web3js-v3-spike.mjs`.
