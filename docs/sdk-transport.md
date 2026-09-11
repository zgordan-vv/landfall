# SDK transport and retries

`sendWithRetry` delegates HTTP/gzip details to an injected transport, retries
server failures up to a bounded attempt count, and applies exponential backoff
with jitter. The same `OutboundBatch` (including its stable batch ID) is passed
to every attempt.
