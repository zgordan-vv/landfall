#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

if ! command -v docker >/dev/null 2>&1; then
    printf 'Docker with Compose v2 is required but the docker command was not found.\n' >&2
    exit 127
fi

if ! docker compose version >/dev/null 2>&1; then
    printf 'Docker Compose v2 is required but `docker compose` is unavailable.\n' >&2
    exit 127
fi

if [[ -n "${LANDFALL_ENV_FILE:-}" ]]; then
    compose_env_file="$LANDFALL_ENV_FILE"
elif [[ -f .env ]]; then
    compose_env_file=.env
else
    compose_env_file=.env.example
    printf 'Using committed local-only database credentials from .env.example.\n' >&2
fi

if [[ ! -f "$compose_env_file" ]]; then
    printf 'Compose environment file does not exist: %s\n' "$compose_env_file" >&2
    exit 2
fi

exec docker compose \
    --project-name landfall \
    --env-file "$compose_env_file" \
    --file docker-compose.yml \
    "$@"
