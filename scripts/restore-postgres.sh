#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${BACKUP_FILE:?Set BACKUP_FILE to a pg_dump custom-format file}"
: "${LANDFALL_RESTORE_DATABASE:?Set LANDFALL_RESTORE_DATABASE explicitly}"
if [[ "${CONFIRM_RESTORE:-}" != "YES" ]]; then
    printf 'Refusing restore: set CONFIRM_RESTORE=YES explicitly.\n' >&2
    exit 2
fi
service="${LANDFALL_DB_SERVICE:-postgres}"
if [[ -n "${LANDFALL_COMPOSE_FILE:-}" ]]; then
    compose=(docker compose)
    [[ -z "${LANDFALL_COMPOSE_PROJECT:-}" ]] || compose+=(--project-name "$LANDFALL_COMPOSE_PROJECT")
    compose+=(--file "$LANDFALL_COMPOSE_FILE")
else
    compose=(bash "$repository_root/scripts/compose.sh")
fi

BACKUP_FILE="$BACKUP_FILE" BACKUP_MANIFEST_FILE="${BACKUP_MANIFEST_FILE:-${BACKUP_FILE}.manifest}" \
    LANDFALL_DB_SERVICE="$service" LANDFALL_COMPOSE_FILE="${LANDFALL_COMPOSE_FILE:-}" \
    LANDFALL_COMPOSE_PROJECT="${LANDFALL_COMPOSE_PROJECT:-}" \
    "$repository_root/scripts/verify-postgres-backup.sh"
"${compose[@]}" exec -T "$service" sh -euc \
    'pg_restore --username "$POSTGRES_USER" --clean --if-exists --no-owner --no-privileges --exit-on-error --dbname "$1"' \
    sh "$LANDFALL_RESTORE_DATABASE" <"$BACKUP_FILE"
printf 'Restore completed into %s on service %s. Run migrations and verification before enabling workers.\n' \
    "$LANDFALL_RESTORE_DATABASE" "$service"
