# Solana Kit integration contract

The adapter instruments an application-owned lifecycle. It observes calls and
results; it does not submit transactions on the application's behalf, manage
keys, or replace customer retry policy.

## Lifecycle

1. Capture `getLatestBlockhash()` and its validity height.
2. Record simulation outcome and compute-unit estimate.
3. Measure application signing latency; fingerprint serialized signed bytes in
   memory and immediately wipe the transient copy.
4. Record each submission attempt with route and preflight configuration.
5. Measure the application's confirmation wait.
6. Send sanitized events asynchronously through the neutral SDK transport.

## Captured fields

| Field | Source | Representation | Privacy rule |
|---|---|---|---|
| `trace_id` | Landfall context | UUIDv7 | Non-secret correlation ID |
| `project_id`, `environment_id` | SDK configuration | UUIDv7 | Public identifiers only |
| `route_id` | Application route config | Bounded string | Endpoint credentials never included |
| `blockhash` | `getLatestBlockhash` result | String | Retained only as lifecycle evidence |
| `last_valid_block_height` | RPC result (`bigint`/string) | Exact decimal string | Never convert through lossy number |
| `simulation.rpc_result` | Simulation response | Protocol enum | Normalize unknown shapes as malformed |
| `simulation.units_consumed` | Simulation response | Exact decimal string | Optional; no raw logs by default |
| `simulation.logs_present` | Simulation response | Boolean | Log contents are not serialized |
| `signing.duration_ns` | Monotonic clock around app signer | Decimal string | No keypair or signer secret access |
| `signed_bytes_fingerprint` | HMAC of exact serialized bytes | Algorithm/key ID/hex digest | Raw bytes are transient and never emitted |
| `attempt_id`, `attempt_sequence` | Submission wrapper | UUIDv7/integer | Stable grouping across retries |
| `encoding`, `skip_preflight`, `max_retries` | Submission config | Enum/boolean/integer | Configuration only, no transaction object |
| `confirmation.result`, `duration_ns` | Confirmation wait wrapper | Enum/decimal string | Original error is sanitized before telemetry |

## Supported lane

P0 targets `@solana/kit` 8.2.0 on Node 24. No Kit plugin is required. The
neutral SDK root remains free of Kit imports; the concrete adapter is the only
place that maps Kit return types into this contract.

See [`config/solana-kit-lane.json`](../config/solana-kit-lane.json) and the
[P0 support matrix](support-matrix.md) for the frozen compatibility claim.
