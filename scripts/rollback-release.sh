#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

: "${LANDFALL_DEPLOY_ENV_FILE:?Set LANDFALL_DEPLOY_ENV_FILE to the deployment environment file}"
: "${LANDFALL_ROLLBACK_IMAGE:?Set LANDFALL_ROLLBACK_IMAGE to the previously verified image digest}"
[[ "$LANDFALL_ROLLBACK_IMAGE" == *@sha256:* ]] || { printf 'Rollback image must be pinned by digest.\n' >&2; exit 2; }

temporary_env="$(mktemp)"
trap 'rm -f "$temporary_env"' EXIT
grep -v '^LANDFALL_SERVER_IMAGE=' "$LANDFALL_DEPLOY_ENV_FILE" >"$temporary_env"
printf 'LANDFALL_SERVER_IMAGE=%s\n' "$LANDFALL_ROLLBACK_IMAGE" >>"$temporary_env"

LANDFALL_DEPLOY_ENV_FILE="$temporary_env" \
LANDFALL_DATABASE_URL_FILE="${LANDFALL_DATABASE_URL_FILE:?Set LANDFALL_DATABASE_URL_FILE}" \
LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE="${LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE:?Set LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE}" \
LANDFALL_DEPLOYMENT_NAME="${LANDFALL_DEPLOYMENT_NAME:-landfall}" \
    "$repository_root/scripts/deploy-release.sh"

printf 'Rollback image applied. Do not roll back database migrations; use a forward-only migration repair.\n'
