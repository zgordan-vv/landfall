# Signed-byte fingerprint boundary

`fingerprintSignedBytesInMemory()` copies the already-signed serialized bytes,
passes the exact copy to the environment-keyed HMAC callback, returns only the
protocol fingerprint, and wipes the transient copy in a `finally` block. The
caller-owned input remains unchanged; raw bytes are not returned or serialized.
