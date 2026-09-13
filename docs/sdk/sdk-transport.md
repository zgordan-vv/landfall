# SDK transport and retries

`createHttpBatchTransport` sends JSON to the configured collector URL plus
`/v1/ingest`, including the stable `batch_id`. `sendWithRetry` retries network failures, HTTP 5xx responses, and
HTTP 429 up to a bounded attempt count with exponential backoff and jitter. The
same `OutboundBatch` (including its stable batch ID) is passed to every attempt.
Other 4xx responses are returned immediately because retrying invalid telemetry
cannot fix it; `LandfallSdk` restores that batch and reports the failure.
