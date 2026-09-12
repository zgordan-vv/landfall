# Metrics guide

Landfall separates outcome metrics from evidence quality so an RPC outage does
not look like a failed transaction.

| Metric family | What it answers | Denominator rule |
|---|---|---|
| Landing rate | Did the transaction reach the chain? | Terminal-eligible traces only |
| Execution success/failure | Did on-chain execution succeed? | Observed execution outcomes |
| Application success | Did the customer application report success? | Application-reported outcomes |
| Unknown/missing | Is evidence incomplete or conflicting? | All traces with missing evidence |

`processed`, `confirmed`, and `finalized` are distinct evidence states, not
interchangeable labels. In-progress, incomplete, and conflicting traces are
excluded from terminal denominators and shown as data-quality warnings. Every
dashboard and report metric uses the versioned
[`metric-definitions.md`](metric-definitions.md) semantics.
