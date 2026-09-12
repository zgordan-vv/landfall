# Authentication security review

## Verification

`cargo test -p landfall-server auth` covers missing, malformed, whitespace,
unknown, revoked, expired, and insufficient-scope credentials.

Bearer tokens are accepted only in exact `Bearer <token>` form. The server
stores SHA-256 digests and compares fixed-size digests using a constant-time
XOR accumulator; plaintext tokens are never persisted or logged. Internal
failure reasons remain distinct so handlers can return generic public errors.

Timing behavior is reviewed at the primitive level; statistically measured
remote timing resistance is outside this unit-test document.
