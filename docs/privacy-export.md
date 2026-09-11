# Privacy Profiles

Report exports accept two profiles:

- `internal` keeps trace identifiers for trusted operators;
- `shareable` replaces trace identifiers with `[redacted]` in JSON and HTML.

State, counts, versions, and aggregate relationships remain available, so a
shareable report can demonstrate findings without exposing transaction
correlation identifiers.
