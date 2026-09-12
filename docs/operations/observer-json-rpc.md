# Provider-neutral observer JSON-RPC

`landfall-observer` exposes `JsonRpcClient<T>` over an injected `RpcTransport`.
The observer therefore knows JSON-RPC 2.0 but not a particular HTTP library or
provider. Responses normalize into three safe error classes: transport
failure, provider error code/message, and malformed response. Provider `data`
is parsed but never copied into the normalized error, preventing unbounded or
secret-bearing payloads from leaking into diagnostics.
