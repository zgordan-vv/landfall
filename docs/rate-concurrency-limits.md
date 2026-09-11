# Rate and concurrency limits

The router enforces a process-wide fixed one-second request window (100
requests) and a maximum of 64 concurrent handlers. Exhausting the rate window
returns structured `429 rate_limited`; concurrency is held by Tower until the
request completes. This is an MVP backpressure boundary; production
deployments should add distributed/token-scoped quotas at the edge.
