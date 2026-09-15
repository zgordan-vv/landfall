# Release and deployment runbook

This is the executable release path for a self-hosted Landfall P0 deployment.
It deliberately separates three decisions: creating a release artifact, proving
it in staging, and changing production. The GitHub workflow publishes an image
and archive; it does not receive SSH credentials and cannot silently change a
customer host.

## 1. Create an immutable release

Merge only after the normal CI pipeline is green. Create and push a signed or
protected version tag such as `v0.1.1`:

```bash
git tag -a v0.1.1 -m 'Landfall v0.1.1'
git push origin v0.1.1
```

The `Release` workflow reruns release gates, including contract, secret,
monitoring and backup/restore checks. It then publishes:

- `ghcr.io/zgordan-vv/landfall` image tags and a digest;
- an SBOM and build provenance attestation attached to the image;
- a GitHub release with source archive, SHA-256 checksum, and provenance file.

Deploy the emitted `ghcr.io/.../landfall@sha256:...` value. Tags such as
`v0.1.1` are convenient for humans but are not deployment inputs because they
can be moved.

Verify a published image before rollout:

```bash
docker buildx imagetools inspect ghcr.io/zgordan-vv/landfall@sha256:RELEASE_DIGEST
gh attestation verify \
  oci://ghcr.io/zgordan-vv/landfall@sha256:RELEASE_DIGEST \
  --owner zgordan-vv
```

The second command requires GitHub CLI authentication. If attestations are not
available to the operator, stop the rollout and investigate the release rather
than substituting a mutable tag.

## 2. Prepare a host

Use distinct staging and production hosts (or at minimum distinct Docker
projects, volumes, secrets, and network boundaries). Install Docker Engine with
Compose v2 and an HTTPS reverse proxy. PostgreSQL is kept on the internal
Compose `data` network; the production overlay binds the application only on
`127.0.0.1:8080`. The TLS proxy on the host is the only public entry point.

Create deployment-only files outside the repository:

```bash
install -d -m 700 /srv/landfall/secrets
install -m 600 /dev/null /srv/landfall/secrets/database-url
install -m 600 /dev/null /srv/landfall/secrets/bootstrap-token
cp deploy/production.env.example /srv/landfall/production.env
chmod 600 /srv/landfall/production.env
```

Put a URL using the internal `postgres-production` hostname in
`database-url`, for example:

```text
postgres://landfall:POSTGRES_PASSWORD@postgres-production:5432/landfall
```

Generate a random bootstrap token, write it to `bootstrap-token`, and replace
every `replace_with_...` value in `production.env`. The bootstrap token is used
only for initial control-plane provisioning; rotate or remove it after that
operation according to [secret rotation](secret-rotation.md). Never put either
secret into the environment file, shell history, Git, screenshots, or chat.

Configure nginx (or an equivalent managed proxy) with
[`config/nginx/landfall.conf.example`](../../config/nginx/landfall.conf.example),
then follow the [TLS runbook](../security/tls-reverse-proxy-runbook.md). Bind
the proxy upstream to the private application network or a host-local listener;
do not reintroduce a public PostgreSQL port.

## 3. Validate and deploy to staging

From the repository checkout on the staging host:

```bash
LANDFALL_DEPLOY_ENV_FILE=/srv/landfall/production.env \
LANDFALL_DATABASE_URL_FILE=/srv/landfall/secrets/database-url \
LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE=/srv/landfall/secrets/bootstrap-token \
  ./scripts/check-production-deployment.sh

LANDFALL_DEPLOYMENT_NAME=landfall-staging \
LANDFALL_DEPLOY_ENV_FILE=/srv/landfall/production.env \
LANDFALL_DATABASE_URL_FILE=/srv/landfall/secrets/database-url \
LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE=/srv/landfall/secrets/bootstrap-token \
  ./scripts/deploy-release.sh
```

Before approving production, run through the TLS proxy:

```bash
curl --fail --silent https://staging.example.com/health/live
curl --fail --silent https://staging.example.com/health/ready
test "$(curl --silent --output /dev/null --write-out '%{http_code}' https://staging.example.com/metrics)" = 404
```

Then complete a scoped authenticated ingest, confirm its trace appears in the
dashboard, confirm observer-worker activity, check Prometheus target status,
and inspect logs for redaction. Run a fresh backup/restore exercise as part of
the staging change when PostgreSQL, migrations, or backup scripts changed.

The GitHub `staging` environment is an approval checkpoint and records the
immutable candidate. Host deployment remains a deliberate operator action.
The deploy script also starts the pinned Prometheus profile; review its target
and rules locally at `http://127.0.0.1:9090` before approval.

## 4. Promote the same digest to production

Copy the *same* image digest, never rebuild or retag it. Repeat validation and
deployment with production files and `LANDFALL_DEPLOYMENT_NAME=landfall`.
The GitHub `production` environment should require an authorized reviewer. Do
not route DNS or customer traffic until `/health/ready`, a scoped ingest, the
dashboard, worker, and Prometheus checks all pass.

Record in the change ticket: release tag, full image digest, Git commit,
migration versions, backup location, approval, start/end time, and rollback
image digest.

## 5. Roll back application code safely

Rollback changes the two application containers only; PostgreSQL migrations are
append-only and must be repaired forward. First stop routing new traffic, take
a backup, identify the last known-good image digest, then run:

```bash
LANDFALL_ROLLBACK_IMAGE=ghcr.io/zgordan-vv/landfall@sha256:KNOWN_GOOD_DIGEST \
LANDFALL_DEPLOYMENT_NAME=landfall \
LANDFALL_DEPLOY_ENV_FILE=/srv/landfall/production.env \
LANDFALL_DATABASE_URL_FILE=/srv/landfall/secrets/database-url \
LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE=/srv/landfall/secrets/bootstrap-token \
  ./scripts/rollback-release.sh
```

Verify `/health/ready`, perform a scoped ingest, and inspect the worker queue
before restoring traffic. If the incident involves data or a migration, use the
[backup and restore runbook](backup-restore-runbook.md) in an isolated recovery
environment rather than attempting to downgrade production data in place.

## Acceptance evidence

A production rollout is complete only with all of the following recorded:

1. release workflow success, image digest, checksum, and provenance verified;
2. staging approval for that exact digest;
3. production Compose validation and all services healthy;
4. TLS live/ready, authenticated ingest, dashboard, observer, and metrics
   smoke tests successful;
5. current backup location and a known-good rollback digest documented.
