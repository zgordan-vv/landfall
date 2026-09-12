# ETag and projection version

Projection-backed reads return a monotonic `projection_version` and a strong
ETag in the form `"projection-{version}"`. The ETag is derived only from the
projection version, so identical versions are cache-safe and every committed
projection advance invalidates the prior response. An exact
`If-None-Match` match produces `304 Not Modified`; missing, weak, or stale
values produce the full response.

## Verification

Run `cargo test -p landfall-server etag`. Only a projection-version advance may
change the ETag.
