# Golden incident runbook

Golden fixtures live under `fixtures/protocol/v1/valid/incidents`. To inspect
one, convert its event batch to NDJSON and pass it to the CLI:

```bash
fixture=fixtures/protocol/v1/valid/incidents/simulation-error.batch.json
jq -c '.events[]' "$fixture" | cargo run -p landfall-cli -- ingest -
```

Useful incidents:

| Fixture | What it demonstrates |
| --- | --- |
| `success.batch.json` | landed and successful execution |
| `simulation-error.batch.json` | confirmed simulation diagnostic |
| `submission-rejection.batch.json` | structured RPC rejection |
| `compute-budget-error.batch.json` | specific compute-budget failure |
| `expiry-without-inclusion.batch.json` | validity-window expiration |
| `timeout-later-success.batch.json` | client timeout followed by success |
| `identical-retry.batch.json` | retry and route-risk signals |
| `missing-block-height.batch.json` | unknown/missing evidence behavior |
| `observer-disagreement.batch.json` | conflicting observer evidence |

Add `--privacy-profile shareable` when demonstrating the report externally.
The command is deterministic and does not require PostgreSQL, an RPC endpoint,
or a running server.
