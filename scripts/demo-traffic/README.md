# Demo traffic

Fills a Landfall demo project with real evidence instead of placeholder rows.
All three scripts post through the public APIs (`/v1/ingest`,
`/v1/x402/authorize`), so projections, diagnostics, and recommendations are
produced by the same code paths a customer would exercise.

| Script                 | What it produces                                                                                       |
| ---------------------- | ------------------------------------------------------------------------------------------------------ |
| `generate.mjs`         | Real Solana transactions on devnet or a local validator, with their measured lifecycle                 |
| `replay-incidents.mjs` | The golden incident fixtures, re-identified and re-timed, in a separate `scenario-library` environment |
| `x402-decisions.mjs`   | Real spend-policy decisions from the x402 authorization endpoint (no payment is made)                  |

The scripts have no dependencies beyond Node.js 20+ (they post directly to
the HTTP API rather than importing the workspace SDK). The generator refuses
to run against mainnet-beta. Its wallet is a disposable keypair holding free
devnet or local-validator SOL.

## Scenarios

Failures come from genuinely misconfigured transactions, not from invented
outcomes. The `baseline` environment mixes all scenarios (about 65% of its
traces run a failure scenario); the optional `candidate` environment runs a
tuned mix (about 30%), so the Comparison view has a difference to show. That
difference is **staged by this generator**, not caused by a real configuration
change, and the environment descriptions must say so (see
[Configure the comparison cohorts](#configure-the-comparison-cohorts)).

| Scenario             | What actually happens                                                     | Expected diagnostic                                    |
| -------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------ |
| `healthy`            | Simulate, sign, submit, confirm, finalize                                 | none                                                   |
| `simulation-error`   | Transfer exceeds the balance; app stops after simulation                  | `RULE-SIM-001`                                         |
| `rpc-rejection`      | Same transfer submitted with preflight; the RPC rejects it                | `RULE-RPC-001`                                         |
| `execution-error`    | Same transfer with `skipPreflight`; it lands and fails on-chain           | `RULE-EXEC-001`                                        |
| `compute-exhaustion` | 300-byte memo (about 107k CU) under a 60k compute limit                   | `RULE-CU-001`                                          |
| `tight-compute`      | Same memo under a 120k limit; succeeds using about 90% of it              | `RULE-CU-002`                                          |
| `stale-blockhash`    | Signed transaction held until its blockhash expires, then rebuilt         | `RULE-EXP-001`, then a replacement trace               |
| `slow-signer`        | Signing step takes 21 seconds                                             | `RULE-SIGN-001`                                        |
| `client-timeout`     | App gives up waiting after 150 ms; the transaction lands anyway           | `RULE-TIMEOUT-001`                                     |
| `flaky-route-retry`  | Two attempts on an unreachable route time out; retry on the primary route | `RULE-RETRY-001`, `RULE-ROUTE-001`, `RULE-TIMEOUT-001` |

Each trace's events are posted once, when the trace finishes. Set
`DEMO_STREAM_EVENTS=1` to post as each phase completes; the dashboard then shows
traces in progress, but the server keeps "missing evidence" findings computed
from the partial snapshots.

## Configuration

| Variable                            | Default                               | Purpose                                              |
| ----------------------------------- | ------------------------------------- | ---------------------------------------------------- |
| `LANDFALL_COLLECTOR_URL`            | `http://127.0.0.1:8080`               | Landfall server                                      |
| `LANDFALL_INGEST_TOKEN`             | none                                  | Token with `ingest:write`                            |
| `LANDFALL_PROJECT_ID`               | required                              | Demo project                                         |
| `LANDFALL_ENVIRONMENT_ID`           | required                              | Baseline environment                                 |
| `LANDFALL_CANDIDATE_ENVIRONMENT_ID` | none                                  | Tuned environment for the Comparison view            |
| `LANDFALL_SCENARIO_ENVIRONMENT_ID`  | required by `replay-incidents.mjs`    | Environment for replayed fixtures                    |
| `LANDFALL_ROUTE_ID`                 | `0198ef00-…-000000000400`             | Registered route of `SOLANA_RPC_URL`                 |
| `LANDFALL_CANDIDATE_ROUTE_ID`       | `LANDFALL_ROUTE_ID`                   | Registered route in the candidate environment        |
| `LANDFALL_FLAKY_ROUTE_ID`           | `0198ef00-…-000000000401`             | Identifier for the unreachable route; not registered |
| `SOLANA_RPC_URL`                    | `http://127.0.0.1:8899`               | Devnet or local validator RPC                        |
| `DEMO_FLAKY_RPC_URL`                | `http://10.255.255.1:8899`            | Unroutable endpoint that produces real timeouts      |
| `DEMO_WALLET`                       | `~/.config/solana/landfall-demo.json` | Disposable keypair                                   |
| `DEMO_STATE_DIR`                    | `~/.landfall-demo-traffic`            | Fingerprint HMAC key and observer identity           |
| `LANDFALL_X402_TOKEN`               | required by `x402-decisions.mjs`      | Token with `x402:pay`                                |

## Set up a demo project

Create the project through the bootstrap API (the server must be started with
`LANDFALL_BOOTSTRAP_TOKEN` or `LANDFALL_BOOTSTRAP_TOKEN_FILE`). Keep the
returned `project:admin` token in a secret store.

```sh
curl -sS -X POST "$LANDFALL_URL/v1/control/projects" \
  -H "Authorization: Bearer $LANDFALL_BOOTSTRAP_TOKEN" -H 'content-type: application/json' \
  --data '{"name":"Landfall demo","initial_token_name":"demo-owner"}'
```

Start the server with `LANDFALL_PUBLIC_DEMO_PROJECT_ID` set to the new
`project_id`, then seed the environments and x402 policy (this also removes the
placeholder rows that earlier versions of the seed script wrote):

```sh
LANDFALL_PUBLIC_DEMO_PROJECT_ID=<project_id> bash scripts/seed-demo-control-plane.sh
```

Create one token per script:

```sh
curl -sS -X POST "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/tokens" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H 'content-type: application/json' \
  --data '{"name":"demo-traffic","scopes":["ingest:write"]}'
curl -sS -X POST "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/tokens" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H 'content-type: application/json' \
  --data '{"name":"demo-x402","scopes":["x402:pay"]}'
```

On devnet, register the RPC route in each live environment so the server's
observer worker independently confirms the transactions, and pass the returned
`route_id` as `LANDFALL_ROUTE_ID` (and the candidate environment's as
`LANDFALL_CANDIDATE_ROUTE_ID`). Routes must use HTTPS, so a local validator
gets no server-side observation unless one is inserted manually.

```sh
curl -sS -X POST "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/environments/$ENVIRONMENT_ID/routes" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H 'content-type: application/json' \
  --data '{"name":"devnet-public","endpoint":"https://api.devnet.solana.com"}'
```

## Configure the comparison cohorts

Comparison never guesses roles from recency: until a baseline and a candidate
are designated, it reports "not configured". `scripts/seed-demo-control-plane.sh`
sets roles and descriptions for the locally seeded environments. For any other
deployment, including the public demo, use the control-plane API with the
project's `project:admin` token after deploying a server that includes migration
`0017_create_comparison_cohorts.sql`.

The public demo at <https://landfall.codehummus.com/#demo> uses project
`01a0a43c-1fd5-7271-a2ca-6d3d20145162` with `devnet` as the baseline
(`01a0a43c-204a-7c22-8d31-38ae595a1e75`) and `devnet-candidate` as the candidate
(`01a0a469-b2f9-7132-a03d-cd9f5b58b8e5`):

```sh
LANDFALL_URL=https://landfall.codehummus.com
PROJECT_ID=01a0a43c-1fd5-7271-a2ca-6d3d20145162
BASELINE_ENVIRONMENT_ID=01a0a43c-204a-7c22-8d31-38ae595a1e75
CANDIDATE_ENVIRONMENT_ID=01a0a469-b2f9-7132-a03d-cd9f5b58b8e5

curl -sS -X PUT "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/environments/$BASELINE_ENVIRONMENT_ID/description" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H 'content-type: application/json' \
  --data '{"description":"Demo: baseline injects failure scenarios (about 65% of traces are deliberately misconfigured by the traffic generator)"}'
curl -sS -X PUT "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/environments/$CANDIDATE_ENVIRONMENT_ID/description" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H 'content-type: application/json' \
  --data '{"description":"Demo: candidate uses tuned settings (about 30% injected failure scenarios); the difference is staged by the traffic generator"}'
curl -sS -X PUT "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/comparison-cohorts" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H 'content-type: application/json' \
  --data '{"baseline_environment_id":"'"$BASELINE_ENVIRONMENT_ID"'","candidate_environment_id":"'"$CANDIDATE_ENVIRONMENT_ID"'"}'

# Check what the public page will show.
curl -sS "$LANDFALL_URL/demo/v1/comparison" | jq '{status, baseline: .baseline.environment, candidate: .candidate.environment, sufficient_sample}'
```

If you change `MIXES` in `generate.mjs`, update both descriptions to match.

## Run

Fund the wallet. Devnet SOL is free from <https://faucet.solana.com> or
`solana airdrop 2 <address> --url devnet`; each demo transaction costs about
0.000005 SOL.

```sh
solana-keygen new --no-bip39-passphrase -o ~/.config/solana/landfall-demo.json
```

Replay the incident library into its own environment. Comparison uses only the
designated baseline and candidate, so the order does not matter.

```sh
node scripts/demo-traffic/replay-incidents.mjs
```

Then generate traffic:

```sh
node scripts/demo-traffic/generate.mjs --all                         # one of each scenario
node scripts/demo-traffic/generate.mjs --once stale-blockhash        # a single scenario
node scripts/demo-traffic/generate.mjs --loop --interval 60          # continuous, both environments
node scripts/demo-traffic/x402-decisions.mjs --count 10
```

Generate a report from the accumulated traces:

```sh
curl -sS -X POST "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/reports" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H 'content-type: application/json' \
  --data '{"title":"Demo devnet reliability report","privacy_profile":"shareable"}'
```

For a long-running demo, run `generate.mjs --loop` under a process supervisor
(for example a systemd service) with the variables above in its environment
file. The public devnet RPC is rate limited; keep `--concurrency` at 3 or
less and `--interval` at 30 seconds or more.
