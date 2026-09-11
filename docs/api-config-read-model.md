# Project, environment, and route configuration

`config_read_model` defines the payload exposed by configuration read
endpoints. It includes stable identifiers, display labels, enabled state,
commitment, privacy mode, and whether route authentication is configured.

Raw RPC URLs, credentials, headers, and tokens are intentionally absent. The
builder sorts environments and routes by identifier so repeated responses are
deterministic for caching and client rendering.
