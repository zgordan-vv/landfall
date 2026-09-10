# Data-quality grading v1

- **Status:** Implemented
- **Version:** `data-quality-v1`
- **Scope:** One canonical transaction trace

The grade measures telemetry completeness and consistency. It does not measure
whether a transaction succeeded, whether an RPC route is reliable, or whether a
business operation produced the intended effect.

## Inputs

The evaluator receives the complete canonically ordered event set, the fresh
trace projection, and retry/replacement grouping output. It never reads a prior
score, PostgreSQL, Solana RPC, or wall-clock `now`, so replay is deterministic.

## Scoring

Every trace starts at 100. Deduplicated findings subtract:

| Severity | Deduction | Meaning |
|---|---:|---|
| `info` | 5 | Optional analytical or correlation evidence is unavailable |
| `warning` | 15 | A supported diagnosis or lifecycle conclusion is materially limited |
| `error` | 40 | Evidence is contradictory or unsafe for automatic correlation |

The score saturates at zero. Grades are:

| Grade | Score |
|---|---:|
| A | 90–100 |
| B | 75–89 |
| C | 60–74 |
| D | 40–59 |
| F | 0–39 |

Repeated reports of the same condition are one finding and one deduction, while
all supporting event IDs remain attached. Attempt-specific missing responses are
separate findings because each represents a distinct telemetry gap.

## Contextual checks

Warnings cover missing trace creation, signing evidence, submission invocation
or response, recent-blockhash validity height, observer coverage, and execution
enrichment. Optional simulation, signed-identity, and business-action correlation
gaps are informational. Signed identity deliberately withheld by strict privacy
is distinguished from accidental telemetry loss.

Expiration evidence is required only for a submitted, not-yet-landed recent-
blockhash trace. Durable-nonce traces are not penalized for lacking
`lastValidBlockHeight`. Execution enrichment is required only after inclusion is
known. This contextual behavior prevents irrelevant fields from lowering every
trace indiscriminately.

Explicit `landfall.data_quality.detected` events retain their producer-assigned
severity. Clock-ordering and grouping warnings are translated into stable
findings. Conflicting signature/fingerprint evidence is an error and blocks safe
automatic identity correlation.

Any change to deductions, thresholds, or contextual rules requires a new version
so historical assessments remain reproducible.
