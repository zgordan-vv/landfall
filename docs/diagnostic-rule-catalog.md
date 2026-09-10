# Diagnostic Rule Catalog

Landfall diagnostic rules are deterministic product logic, not generated prose.
The initial rule set is `diagnostic-rules-v1`.

Each finding records:

- a stable rule ID;
- the rule-set version;
- a machine-readable claim key;
- certainty for that exact claim;
- immutable source-event evidence.

## Confirmed Rules

### RULE-SIM-001 — Simulation error

Confirms that simulation returned a direct non-success RPC result:
`execution_error`, `blockhash_not_found`, `node_error`, or
`malformed_response`.

Result: `simulation_error`.

Why: this is direct local/RPC evidence that simulation did not succeed.

### RULE-RPC-001 — Structured RPC submission rejection

Confirms that a submission route returned a structured rejection result, such as
`rejected`, `blockhash_rejected`, `duplicate_signature`, `already_processed`,
`rate_limited`, `unauthorized`, or `malformed_response`.

Result: `rpc_submission_rejection`.

Why: the route explicitly answered with a bounded rejection category.

### RULE-EXEC-001 — On-chain execution error

Confirms that an observed included transaction executed with
`execution_result = failure`, except when the direct error category is compute
budget exhaustion.

Result: `on_chain_execution_error`.

Why: inclusion and execution failure are direct on-chain evidence.

### RULE-CU-001 — Compute-budget failure

Confirms compute-budget exhaustion when simulation or on-chain execution evidence
contains `compute_budget_exceeded`.

Result: `compute_budget_failure`.

Why: compute exhaustion is more specific than a generic execution or simulation
failure and should not be duplicated as both categories for one event.

### RULE-EXP-001 — Validity window passed

Confirms that the reducer classified the trace as expired without observed
inclusion, using the latest not-found status observation as evidence.

Result: `expired_without_observed_inclusion`.

Why: the configured observer policy reached a block-height boundary beyond
`lastValidBlockHeight` without seeing inclusion. This confirms the local
observation state, not the validator-internal cause.

### RULE-TIMEOUT-001 — Client timeout followed by network success

Confirms that a client-side submission or confirmation wait timed out, and later
status/execution evidence showed network success.

Result: `client_timeout_followed_by_network_success`.

Why: client timeout and network success are separate facts. This prevents a UI or
report from calling the transaction failed just because the application stopped
waiting.
