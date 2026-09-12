# API documentation guide

The REST contract is generated from Rust handlers with Utoipa and is exposed as
`GET /api/openapi.json` when the runtime server is wired. Endpoint-specific
contracts are listed in this directory; the [OpenAPI document](openapi.md) is
the starting point for generated clients.

API consumers must send authentication in the `Authorization` header, scope
queries by project/environment, treat ETags and projection versions as cache
validators, and preserve cursor values exactly. `Unknown` and data-quality
fields are part of the contract and must not be collapsed into success/failure.
