# Authentication security review

Bearer tokens are accepted only in the exact `Bearer <token>` form, with no
whitespace in the token. The server hashes the presented token with SHA-256
and compares the fixed-size digests using a constant-time XOR accumulator; it
never stores or logs plaintext tokens. Revoked, expired, unknown, malformed,
and insufficient-scope credentials have distinct internal outcomes while
public handlers can map them to safe generic errors.
