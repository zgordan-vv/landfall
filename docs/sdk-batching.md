# SDK batching and retries

`BatchAssembler` drains up to a configured batch size from `EventBuffer` and
assigns one UUIDv7 `batchId`. `retry` reuses that ID and the exact event order,
allowing the collector to deduplicate transport retries safely.
