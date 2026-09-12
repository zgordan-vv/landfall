#!/usr/bin/env bash
set -euo pipefail

: "${BACKUP_FILE:?Set BACKUP_FILE to the .dump output path}"
service="${LANDFALL_DB_SERVICE:-postgres}"
umask 077
mkdir -p "$(dirname "$BACKUP_FILE")"
docker compose exec -T "$service" pg_dump --no-owner --no-privileges --format=custom --file=- >"$BACKUP_FILE"
bytes=$(wc -c <"$BACKUP_FILE" | tr -d ' ')
sha256=$(shasum -a 256 "$BACKUP_FILE" | awk '{print $1}')
printf 'file=%s\nbytes=%s\nsha256=%s\nservice=%s\n' "$BACKUP_FILE" "$bytes" "$sha256" "$service" >"${BACKUP_FILE}.manifest"
printf 'Backup written to %s\nManifest written to %s\n' "$BACKUP_FILE" "${BACKUP_FILE}.manifest"
