#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

compose=(bash scripts/compose.sh)
container_id="$("${compose[@]}" ps --quiet postgres)"

if [[ -z "$container_id" ]]; then
    printf 'The Landfall PostgreSQL container is not running.\n' >&2
    exit 1
fi

health_status="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$container_id")"

if [[ "$health_status" != "healthy" ]]; then
    printf 'PostgreSQL health status is %s, expected healthy.\n' "$health_status" >&2
    exit 1
fi

server_version_number="$("${compose[@]}" exec --no-TTY postgres sh -euc \
    'psql --no-psqlrc --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" --tuples-only --no-align --command "SHOW server_version_num"')"

if [[ "$server_version_number" != "180006" ]]; then
    printf 'PostgreSQL version mismatch: expected server_version_num 180006, found %s.\n' \
        "$server_version_number" >&2
    exit 1
fi

data_checksums="$("${compose[@]}" exec --no-TTY postgres sh -euc \
    'psql --no-psqlrc --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" --tuples-only --no-align --command "SHOW data_checksums"')"

if [[ "$data_checksums" != "on" ]]; then
    printf 'PostgreSQL data checksums are %s, expected on.\n' "$data_checksums" >&2
    exit 1
fi

published_ports="$(docker inspect --format '{{range $port, $bindings := .NetworkSettings.Ports}}{{if $bindings}}{{$port}} {{end}}{{end}}' "$container_id")"

if [[ -n "$published_ports" ]]; then
    printf 'PostgreSQL unexpectedly publishes host ports: %s\n' "$published_ports" >&2
    exit 1
fi

network_names="$(docker inspect --format '{{range $name, $network := .NetworkSettings.Networks}}{{$name}}{{"\n"}}{{end}}' "$container_id")"

if [[ -z "$network_names" ]]; then
    printf 'PostgreSQL is not attached to a Compose network.\n' >&2
    exit 1
fi

while IFS= read -r network_name; do
    if [[ "$(docker network inspect --format '{{.Internal}}' "$network_name")" != "true" ]]; then
        printf 'PostgreSQL network is not private/internal: %s\n' "$network_name" >&2
        exit 1
    fi
done <<< "$network_names"

data_mount="$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql"}}{{.Type}}:{{.Name}}{{end}}{{end}}' "$container_id")"

if [[ "$data_mount" != volume:* ]]; then
    printf 'PostgreSQL data is not backed by the expected named volume.\n' >&2
    exit 1
fi

printf 'PostgreSQL 18.6 is healthy, checksummed, private, and backed by %s.\n' "$data_mount"
