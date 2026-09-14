# Project, environment, and route configuration

`GET /v1/control/projects/{project_id}/config` returns the authenticated
project's environments and routes. It includes stable identifiers, display
labels, Solana cluster, enabled state, and whether a route has a configured
endpoint.

Raw RPC URLs, credentials, headers, and tokens are intentionally absent. The
builder sorts environments and routes by identifier so repeated responses are
deterministic for caching and client rendering.
