#!/usr/bin/env bash

# Creates one verified PostgreSQL archive for a scheduled job, then prunes only
# Landfall archives older than the configured retention period. This script is
# deliberately host-local: copying the verified archive to encrypted off-host
# storage is a separate operator responsibility.
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${LANDFALL_BACKUP_DIR:?Set LANDFALL_BACKUP_DIR to an owner-only backup directory}"
: "${LANDFALL_ENV_FILE:?Set LANDFALL_ENV_FILE to the Compose environment file}"

retention_days="${LANDFALL_BACKUP_RETENTION_DAYS:-7}"
minimum_free_kb="${LANDFALL_BACKUP_MIN_FREE_KB:-1048576}"
[[ "$retention_days" =~ ^[1-9][0-9]*$ ]] || { printf 'LANDFALL_BACKUP_RETENTION_DAYS must be a positive integer.\n' >&2; exit 2; }
[[ "$minimum_free_kb" =~ ^[1-9][0-9]*$ ]] || { printf 'LANDFALL_BACKUP_MIN_FREE_KB must be a positive integer.\n' >&2; exit 2; }
[[ -r "$LANDFALL_ENV_FILE" ]] || { printf 'Compose environment file is not readable.\n' >&2; exit 2; }

umask 077
install -d -m 700 "$LANDFALL_BACKUP_DIR"
available_kb="$(df -Pk "$LANDFALL_BACKUP_DIR" | awk 'NR == 2 { print $4 }')"
[[ "$available_kb" =~ ^[0-9]+$ && "$available_kb" -ge "$minimum_free_kb" ]] || {
    printf 'Refusing backup: free disk space is below the configured safety floor.\n' >&2
    exit 1
}

lock_directory="$LANDFALL_BACKUP_DIR/.scheduled-backup.lock"
if ! mkdir "$lock_directory" 2>/dev/null; then
    printf 'A scheduled backup is already running; skipping overlapping run.\n' >&2
    exit 0
fi
trap 'rmdir "$lock_directory"' EXIT

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
backup_file="$LANDFALL_BACKUP_DIR/landfall-$timestamp.dump"
LANDFALL_ENV_FILE="$LANDFALL_ENV_FILE" BACKUP_FILE="$backup_file" \
    "$repository_root/scripts/backup-postgres.sh"

find "$LANDFALL_BACKUP_DIR" -maxdepth 1 -type f \
    \( -name 'landfall-*.dump' -o -name 'landfall-*.dump.manifest' \) \
    -mtime "+$retention_days" -print -delete
printf 'Scheduled backup completed: %s (retention: %s days).\n' "$backup_file" "$retention_days"
