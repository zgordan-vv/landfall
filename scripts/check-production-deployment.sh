#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

: "${LANDFALL_DEPLOY_ENV_FILE:?Set LANDFALL_DEPLOY_ENV_FILE to the deployment environment file}"
: "${LANDFALL_DATABASE_URL_FILE:?Set LANDFALL_DATABASE_URL_FILE to the database URL secret file}"
: "${LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE:?Set LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE to the bootstrap token secret file}"

[[ -r "$LANDFALL_DEPLOY_ENV_FILE" ]] || { printf 'Deployment environment file is not readable.\n' >&2; exit 2; }
[[ -s "$LANDFALL_DATABASE_URL_FILE" ]] || { printf 'Database URL secret file is missing or empty.\n' >&2; exit 2; }
[[ -s "$LANDFALL_BOOTSTRAP_TOKEN_SECRET_FILE" ]] || { printf 'Bootstrap token secret file is missing or empty.\n' >&2; exit 2; }

image_reference="$(sed -nE 's/^LANDFALL_SERVER_IMAGE=(.+)$/\1/p' "$LANDFALL_DEPLOY_ENV_FILE" | tail -n 1)"
[[ "$image_reference" == *@sha256:* ]] || {
    printf 'LANDFALL_SERVER_IMAGE must be a registry image pinned by digest.\n' >&2
    exit 2
}

compose_files=(--file docker-compose.yml --file docker-compose.production.yml)
if [[ "${LANDFALL_DEPLOY_TARGET:-self-hosted-postgres}" == "managed-postgres" ]]; then
    compose_files+=(--file docker-compose.digitalocean.yml)
fi

docker compose --env-file "$LANDFALL_DEPLOY_ENV_FILE" "${compose_files[@]}" \
    --profile production --profile monitoring config --quiet

rendered="$(docker compose \
    --env-file "$LANDFALL_DEPLOY_ENV_FILE" \
    "${compose_files[@]}" \
    --profile production \
    --profile monitoring config)"

grep -Fq "image: $image_reference" <<<"$rendered" || { printf 'Rendered Compose configuration does not use the requested release image.\n' >&2; exit 1; }
grep -Fq 'read_only: true' <<<"$rendered" || { printf 'Production services must use a read-only root filesystem.\n' >&2; exit 1; }
if awk '/^  server:/{inside=1; next} /^  [a-zA-Z]/{inside=0} inside && /^    build:/{found=1} END{exit found ? 0 : 1}' <<<"$rendered"; then
    printf 'Production server must not build an image from the host checkout.\n' >&2
    exit 1
fi

if [[ "${LANDFALL_DEPLOY_TARGET:-self-hosted-postgres}" == "managed-postgres" ]] \
    && ! awk '/^  postgres-production:/{inside=1; next} /^  [a-zA-Z-]+:/{inside=0} inside && /self-hosted-postgres/{found=1} END{exit found ? 0 : 1}' <<<"$rendered"; then
    printf 'Managed PostgreSQL target must keep the local PostgreSQL service out of the production profile.\n' >&2
    exit 1
fi
if ! grep -Fq 'published: "8080"' <<<"$rendered" || ! grep -Fq 'host_ip: 127.0.0.1' <<<"$rendered"; then
    printf 'Production server must publish port 8080 on loopback only for the TLS proxy.\n' >&2
    exit 1
fi

printf 'Production Compose configuration is valid, uses an immutable image, and exposes the server only on loopback for TLS termination.\n'
