# OpenAPI and TypeScript API client

`landfall-server::ApiDoc` now registers the Phase 11 read-model schemas so
`GET /openapi.json` describes metrics, data quality, comparisons,
recommendations, configuration, and system status payloads. The
`@landfall/api-client` package provides a small typed fetch client for the
metric, data-quality, and authenticated system-status endpoints. Applications
may inject their own fetch implementation for browser, Node, or test usage.

## Verification

Run `./node_modules/.bin/tsc -b packages/api-client --pretty false`. Tokens are
kept only in memory and sent through the Authorization header.
