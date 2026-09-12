# Privacy field matrix

Use this as the fast review sheet; the detailed classification and machine-
readable registry remain authoritative in
[event-privacy-classification.md](event-privacy-classification.md) and
[event-privacy-classification.json](event-privacy-classification.json).

| Data category | Default action | Examples |
|---|---|---|
| Landfall identifiers | Store, scope by environment | `project_id`, `trace_id`, `event_id` |
| Timing and normalized telemetry | Store | durations, result enums, versions |
| Public chain identifiers | Store in standard mode; local-only in strict mode | signature, recent blockhash |
| Pseudonymous fingerprints | Store scoped with algorithm/key ID | signed-byte HMAC |
| Bounded diagnostic text | Redact, then store | sanitized error message |
| Secrets and raw transaction bytes | Never collect | private keys, seed phrases, serialized bytes |

Before adding a field, classify it, define its mode behavior, add it to the
registry, and run the privacy/contract checks. A field being publicly visible
on-chain does not make it harmless to retain: combinations can reveal customer
activity or strategy.
