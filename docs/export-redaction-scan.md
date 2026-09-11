# Export redaction and secret scan

Before an artifact is published, `scan_export` checks recursively for
credential-bearing keys (`private_key`, bearer authorization, RPC URLs, and
similar material), PEM blocks, and suspiciously large scalar values. The
export path can then apply `redact_export`, which replaces sensitive fields with
`[redacted]` while preserving the JSON shape for consumers.
