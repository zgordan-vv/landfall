# SDK batching and retries

`BatchAssembler` drains up to a configured batch size from `EventBuffer` and
assigns one UUIDv7 `batchId`. `retry` reuses that ID and the exact event order,
allowing the collector to deduplicate transport retries safely.

The SDK keeps the in-flight batch until the collector responds with a 2xx
status. A retry uses the exact same event order and batch identity. Non-2xx
responses return the batch to the buffer; events are never silently discarded.
