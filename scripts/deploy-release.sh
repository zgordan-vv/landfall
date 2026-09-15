#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

: "${LANDFALL_DEPLOY_ENV_FILE:?Set LANDFALL_DEPLOY_ENV_FILE to the deployment environment file}"
: "${LANDFALL_DATABASE_URL_FILE:?Set LANDFALL_DATABASE_URL_FILE to the database URL secret file}"
: "${LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE:?Set LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE to the bootstrap token secret file}"
deployment_name="${LANDFALL_DEPLOYMENT_NAME:-landfall}"

LANDFALL_DEPLOY_ENV_FILE="$LANDFALL_DEPLOY_ENV_FILE" \
LANDFALL_DATABASE_URL_FILE="$LANDFALL_DATABASE_URL_FILE" \
LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE="$LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE" \
    "$repository_root/scripts/check-production-deployment.sh"

compose=(docker compose --project-name "$deployment_name" --env-file "$LANDFALL_DEPLOY_ENV_FILE" \
    --file docker-compose.yml --file docker-compose.production.yml)
if [[ "${LANDFALL_DEPLOY_TARGET:-self-hosted-postgres}" == "managed-postgres" ]]; then
    compose+=(--file docker-compose.digitalocean.yml)
fi
compose+=(--profile production --profile monitoring)

"${compose[@]}" pull server observer-worker prometheus
if [[ "${LANDFALL_DEPLOY_TARGET:-self-hosted-postgres}" != "managed-postgres" ]]; then
    "${compose[@]}" up --detach --wait postgres-production
fi
"${compose[@]}" up --detach --no-deps server observer-worker prometheus

printf 'Deployment applied. Verify /health/ready through the trusted TLS proxy before routing traffic.\n'
