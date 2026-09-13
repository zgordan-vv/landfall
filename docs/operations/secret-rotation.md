# Secret injection and rotation

## Starting production Compose safely

The repository's `.env.example` is for local development only. Keep real
configuration outside Git:

1. Copy `.env.production.example` to a private deployment directory and set
   its permissions to `0600`.
2. Create `database-url.txt` and `bootstrap-token.txt` in that directory, both
   with mode `0600`. The database file's only
   content is the full PostgreSQL connection URL, for example
   `postgres://landfall_runtime:...@postgres-production:5432/landfall`.
   The bootstrap file holds the one deployment-level bearer credential used to
   provision the first project. It is not a customer API token.
3. Start the production overlay:

   ```sh
   LANDFALL_DATABASE_URL_FILE=/secure/landfall/database-url.txt \
   LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE=/secure/landfall/bootstrap-token.txt \
   docker compose --env-file /secure/landfall/.env.production \
     -f docker-compose.yml -f docker-compose.production.yml \
     --profile production up -d --build
   ```

`docker-compose.production.yml` mounts these files as Docker secrets. The
server and worker use `DATABASE_URL_FILE`, while the server reads
`LANDFALL_BOOTSTRAP_TOKEN_FILE` only to authorize first-project provisioning.
The connection string and bootstrap credential are therefore not in the
application service definition, command line, or repository.

## Rotation procedure

For database credentials, create a new PostgreSQL role or password first and
update `database-url.txt`. Redeploy the two application services, verify
`/health/ready` and one authenticated request, then revoke the old database
credential. Keep the overlap short.

For API tokens, create a replacement token with the same project and scopes,
move each caller to it, and revoke the old token only after its last consumer
has switched. Store only a token prefix or database role name in change records;
never record the plaintext secret.
