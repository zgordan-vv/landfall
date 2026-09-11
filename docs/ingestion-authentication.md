# Ingestion authentication policy

The server never stores or compares plaintext API tokens. A presented
`Authorization: Bearer <token>` value is SHA-256 hashed and compared in
constant time against the stored 32-byte digest. Authorization then checks
revocation, expiration, and the required scope. Each failure has a distinct
internal `AuthError`, allowing the HTTP layer to map it to a stable response
without leaking whether an unrelated token exists.
