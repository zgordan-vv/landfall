# Report artifact storage

The artifact store accepts JSON or HTML bytes only up to 10 MiB, records the
format/privacy profile, byte count, and SHA-256 digest, and exposes immutable
content for download. Verification recomputes the digest before serving bytes;
modified or truncated content fails with `ChecksumMismatch`.
