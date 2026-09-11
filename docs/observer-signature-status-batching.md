# Signature-status batching

`get_signature_statuses()` groups signatures into chunks of at most 256, the
P0 RPC request limit, and concatenates each response in the same order as the
input list. Empty input returns immediately; oversized workloads are split
without unbounded request payloads.
