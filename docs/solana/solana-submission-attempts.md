# Solana submission attempts

`SubmissionAttemptConfig` makes each send explicit: stable attempt ID, route
ID, attempt sequence, wire encoding, and preflight policy. `submitWithRoute()`
executes the application-owned send operation and returns either its original
value or original error together with the configuration. The wrapper does not
implement hidden retries or alter customer RPC behavior.
