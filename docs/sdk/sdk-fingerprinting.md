# SDK signed-byte fingerprinting

`fingerprintSignedBytes` accepts only the exact serialized signed transaction
bytes as `Uint8Array`. It invokes an environment-keyed HMAC callback and emits
the protocol's `lf-hmac-sha256-v1` 32-byte lowercase hex fingerprint. JSON,
base64 strings, unsigned messages, and transaction objects are deliberately not
accepted by the API.
