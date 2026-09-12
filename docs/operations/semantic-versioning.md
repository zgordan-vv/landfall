# Semantic Versioning Manifest

Every derived result is interpreted with four explicit versions:

- `trace-reducer-v1` — event replay and state projection;
- `diagnostic-rules-v1` — confirmed, probable, and unknown findings;
- `metric-definitions-v1` — metric flags and denominators;
- `recommendation-rules-v1` — advisory mapping.

`CoreVersionManifest` exposes these values together. Consumers may compare two
results only when all fields match; otherwise they must display the versions or
apply an explicit migration rather than silently mixing semantics.
