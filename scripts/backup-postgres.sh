#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${BACKUP_FILE:?Set BACKUP_FILE to the new .dump output path}"
service="${LANDFALL_DB_SERVICE:-postgres}"
manifest_file="${BACKUP_MANIFEST_FILE:-${BACKUP_FILE}.manifest}"

if [[ -n "${LANDFALL_COMPOSE_FILE:-}" ]]; then
    compose=(docker compose)
    [[ -z "${LANDFALL_COMPOSE_PROJECT:-}" ]] || compose+=(--project-name "$LANDFALL_COMPOSE_PROJECT")
    compose+=(--file "$LANDFALL_COMPOSE_FILE")
else
    compose=(bash "$repository_root/scripts/compose.sh")
fi

sha256_file() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'; else shasum -a 256 "$1" | awk '{print $1}'; fi
}

[[ ! -e "$BACKUP_FILE" ]] || { printf 'Refusing to overwrite backup: %s\n' "$BACKUP_FILE" >&2; exit 2; }
[[ ! -e "$manifest_file" ]] || { printf 'Refusing to overwrite manifest: %s\n' "$manifest_file" >&2; exit 2; }
umask 077
mkdir -p "$(dirname "$BACKUP_FILE")" "$(dirname "$manifest_file")"
temporary_backup="$(mktemp "${BACKUP_FILE}.tmp.XXXXXX")"
temporary_manifest="$(mktemp "${manifest_file}.tmp.XXXXXX")"
trap 'rm -f "$temporary_backup" "$temporary_manifest"' EXIT

"${compose[@]}" exec -T "$service" sh -euc \
    'pg_dump --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" --no-owner --no-privileges --format=custom' \
    >"$temporary_backup"
bytes="$(wc -c <"$temporary_backup" | tr -d ' ')"
sha256="$(sha256_file "$temporary_backup")"
postgres_version="$("${compose[@]}" exec -T "$service" sh -euc 'psql --no-psqlrc --username "$POSTGRES_USER" --dbname "$POSTGRES_DB" --tuples-only --no-align --command "SHOW server_version_num"')"
printf 'manifest_version=1\narchive_format=custom\nbytes=%s\nsha256=%s\npostgres_version=%s\nservice=%s\ncreated_at=%s\n' \
    "$bytes" "$sha256" "$postgres_version" "$service" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >"$temporary_manifest"
mv "$temporary_backup" "$BACKUP_FILE"
mv "$temporary_manifest" "$manifest_file"
trap - EXIT

BACKUP_FILE="$BACKUP_FILE" BACKUP_MANIFEST_FILE="$manifest_file" LANDFALL_DB_SERVICE="$service" \
    LANDFALL_COMPOSE_FILE="${LANDFALL_COMPOSE_FILE:-}" LANDFALL_COMPOSE_PROJECT="${LANDFALL_COMPOSE_PROJECT:-}" \
    "$repository_root/scripts/verify-postgres-backup.sh"
printf 'Backup written to %s\nManifest written to %s\n' "$BACKUP_FILE" "$manifest_file"
