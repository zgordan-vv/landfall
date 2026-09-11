# OpenAPI contract

The server generates OpenAPI from the Rust request/response types and route
annotations. `GET /openapi.json` exposes the versioned document, including
the ingestion request shape and `200`/`202`/`400`/`413` responses. A test keeps
the route and core schemas present, preventing accidental contract drift.
