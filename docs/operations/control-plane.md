# Project provisioning and token management

Landfall's control-plane API lets an operator create a customer project without
direct PostgreSQL access. It is intentionally separate from telemetry access:
the deployment-level bootstrap credential creates the first project, and that
project's administrator token manages only its own environments and tokens.

## Create the first project

Use the content of `bootstrap-token.txt` once. The response contains the first
project administrator token. Save it in a secret manager immediately: Landfall
stores only its SHA-256 hash and will never return the plaintext value again.

```sh
curl -sS -X POST "$LANDFALL_URL/v1/control/projects" \
  -H "Authorization: Bearer $LANDFALL_BOOTSTRAP_TOKEN" \
  -H 'Content-Type: application/json' \
  --data '{"name":"Acme payments","initial_token_name":"acme-owner"}'
```

The bootstrap endpoint is unavailable unless `LANDFALL_BOOTSTRAP_TOKEN_FILE`
or `LANDFALL_BOOTSTRAP_TOKEN` is configured. Production Compose supplies the
file-backed variant. Rotate this credential after initial provisioning or keep
it in an operator-only secret store for future project creation.

## Add environments

Use the returned `project:admin` token. Environment names are unique inside a
project. `cluster` is a customer-visible label such as `mainnet-beta`,
`devnet`, or `staging`.

```sh
curl -sS -X POST "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/environments" \
  -H "Authorization: Bearer $LANDFALL_PROJECT_ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  --data '{"name":"production","cluster":"mainnet-beta"}'
```

List environments with `GET /v1/control/projects/$PROJECT_ID/environments`.

## Create and revoke service tokens

Issue the smallest scope set needed by an integration:

- `ingest:write` — SDK/collector event submission;
- `traces:read` — trace list, detail, overview, comparison;
- `diagnostics:read` — diagnostics and recommendations;
- `admin` — system status;
- `project:admin` — project environment and token management.

```sh
curl -sS -X POST "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/tokens" \
  -H "Authorization: Bearer $LANDFALL_PROJECT_ADMIN_TOKEN" \
  -H 'Content-Type: application/json' \
  --data '{"name":"sdk-production","scopes":["ingest:write"]}'
```

The token is returned only by this creation response. List safe metadata with
`GET /v1/control/projects/$PROJECT_ID/tokens`; no plaintext token or hash is
included. Revoke a compromised or retired token with:

```sh
curl -sS -X POST \
  "$LANDFALL_URL/v1/control/projects/$PROJECT_ID/tokens/$TOKEN_ID/revoke" \
  -H "Authorization: Bearer $LANDFALL_PROJECT_ADMIN_TOKEN"
```

Revocation takes effect on the next request because bearer tokens are checked
against the database on every protected API call.
