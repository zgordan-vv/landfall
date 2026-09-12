# Contributing

Start with [CONTRIBUTING.md](../CONTRIBUTING.md) for repository setup and the
required checks. A normal change should run:

```bash
just check
git diff --check
```

Keep dependency direction intact, add a named test for every invariant or
failure boundary, update the relevant contract/runbook, and include migration
or privacy implications in the review description. Do not add secrets,
private keys, raw signed bytes, or customer data to fixtures or logs.
