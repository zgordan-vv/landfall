#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
compose_file="$repository_root/deploy/testing/backup-restore.compose.yml"
project_name="landfall-backup-e2e"
backup_directory="$(mktemp -d)"
compose=(docker compose --project-name "$project_name" --file "$compose_file")

cleanup() {
    "${compose[@]}" down --volumes --remove-orphans >/dev/null 2>&1 || true
    rm -rf "$backup_directory"
}
trap cleanup EXIT

"${compose[@]}" up --detach --wait
for attempt in {1..15}; do
    if "${compose[@]}" exec -T postgres-source psql --username=backup_e2e --dbname=backup_e2e \
        --command "CREATE TABLE restore_proof (id integer PRIMARY KEY, value text NOT NULL); INSERT INTO restore_proof VALUES (1, 'restored');"; then
        break
    fi
    [[ "$attempt" -lt 15 ]] || { printf 'Source database did not become queryable after its health check.\n' >&2; exit 1; }
    sleep 1
done

BACKUP_FILE="$backup_directory/landfall.dump" \
LANDFALL_DB_SERVICE=postgres-source \
LANDFALL_COMPOSE_FILE="$compose_file" \
LANDFALL_COMPOSE_PROJECT="$project_name" \
    "$repository_root/scripts/backup-postgres.sh"

CONFIRM_RESTORE=YES \
BACKUP_FILE="$backup_directory/landfall.dump" \
LANDFALL_DB_SERVICE=postgres-restore \
LANDFALL_RESTORE_DATABASE=backup_e2e \
LANDFALL_COMPOSE_FILE="$compose_file" \
LANDFALL_COMPOSE_PROJECT="$project_name" \
    "$repository_root/scripts/restore-postgres.sh"

restored_value="$("${compose[@]}" exec -T postgres-restore psql --username=backup_e2e --dbname=backup_e2e --tuples-only --no-align --command 'SELECT value FROM restore_proof WHERE id = 1')"
[[ "$restored_value" == 'restored' ]] || { printf 'Restored data did not match the source.\n' >&2; exit 1; }
printf 'Backup/restore end-to-end check passed.\n'
