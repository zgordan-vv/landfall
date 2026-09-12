#!/usr/bin/env bash
set -euo pipefail

: "${BACKUP_FILE:?Set BACKUP_FILE to a pg_dump custom-format file}"
if [[ "${CONFIRM_RESTORE:-}" != "YES" ]]; then
  echo 'Refusing restore: set CONFIRM_RESTORE=YES explicitly.' >&2
  exit 2
fi
[[ -r "$BACKUP_FILE" ]] || { echo "Backup is not readable: $BACKUP_FILE" >&2; exit 2; }
service="${LANDFALL_DB_SERVICE:-postgres}"
docker compose exec -T "$service" pg_restore --clean --if-exists --no-owner --no-privileges --exit-on-error --dbname="${POSTGRES_DB:-landfall}" <"$BACKUP_FILE"
echo "Restore completed from $BACKUP_FILE; run migrations and verification before enabling workers."
