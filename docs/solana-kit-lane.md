# Solana Kit P0 lane

The first adapter lane is deliberately narrow: `@solana/kit` **8.2.0** on
Node.js `>=24.11.0 <25.0.0`, with no external Kit plugins. The authoritative
machine-readable declaration is [`config/solana-kit-lane.json`](../config/solana-kit-lane.json).

Verify it with:

```sh
node scripts/check-solana-kit-lane.mjs
```

Later Kit versions are not automatically supported. A dependency update must
pass the complete adapter suite and update the support matrix in the same
reviewed change.
