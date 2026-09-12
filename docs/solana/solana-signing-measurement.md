# Signing delay measurement

`measureSigning()` wraps the application's signing promise and measures its
duration using an injected monotonic `bigint` clock. The adapter never receives
a keypair or signer callback and does not inspect the signed value. Durations
are emitted as exact decimal nanoseconds; both successful values and original
errors remain available to the caller.
