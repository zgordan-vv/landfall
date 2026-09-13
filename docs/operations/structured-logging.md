# Structured production logging

Landfall writes JSON Lines to stderr for both process roles: `server` and
`observer-worker`. Every line includes a timestamp, level, target, message, and
the current span. HTTP completion events additionally include only
`request_id`, method, path, and status.

Request bodies, query strings, `Authorization` headers, API token plaintext,
RPC endpoints, signatures, wallet addresses, and raw transaction data are not
logging fields. Startup errors use a fixed `error_category` instead of printing
configuration or driver error text.

Set `RUST_LOG` to control volume without changing the output format:

```sh
RUST_LOG=landfall_server=debug bash scripts/compose.sh up -d
docker compose logs --no-log-prefix server | jq -c .
```

Use the request ID from an API response's `x-request-id` header to find the
corresponding JSON log event. Preserve logs according to the deployment's
privacy policy; they are operational evidence, not a substitute for durable
trace events.
