#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
: "${BACKUP_FILE:?Set BACKUP_FILE to a pg_dump custom-format file}"
manifest_file="${BACKUP_MANIFEST_FILE:-${BACKUP_FILE}.manifest}"
service="${LANDFALL_DB_SERVICE:-postgres}"

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

[[ -r "$BACKUP_FILE" ]] || { printf 'Backup is not readable: %s\n' "$BACKUP_FILE" >&2; exit 2; }
[[ -r "$manifest_file" ]] || { printf 'Manifest is not readable: %s\n' "$manifest_file" >&2; exit 2; }

manifest_version=''; archive_format=''; expected_bytes=''; expected_sha256=''; postgres_version=''; manifest_service=''; created_at=''
while IFS='=' read -r key value; do
    case "$key" in
        manifest_version) [[ -z "$manifest_version" ]] || { printf 'Manifest repeats manifest_version.\n' >&2; exit 2; }; manifest_version="$value" ;;
        archive_format) [[ -z "$archive_format" ]] || { printf 'Manifest repeats archive_format.\n' >&2; exit 2; }; archive_format="$value" ;;
        bytes) [[ -z "$expected_bytes" ]] || { printf 'Manifest repeats bytes.\n' >&2; exit 2; }; expected_bytes="$value" ;;
        sha256) [[ -z "$expected_sha256" ]] || { printf 'Manifest repeats sha256.\n' >&2; exit 2; }; expected_sha256="$value" ;;
        postgres_version) [[ -z "$postgres_version" ]] || { printf 'Manifest repeats postgres_version.\n' >&2; exit 2; }; postgres_version="$value" ;;
        service) [[ -z "$manifest_service" ]] || { printf 'Manifest repeats service.\n' >&2; exit 2; }; manifest_service="$value" ;;
        created_at) [[ -z "$created_at" ]] || { printf 'Manifest repeats created_at.\n' >&2; exit 2; }; created_at="$value" ;;
        *) printf 'Manifest has an unknown key: %s\n' "$key" >&2; exit 2 ;;
    esac
done <"$manifest_file"

[[ "$manifest_version" == '1' ]] || { printf 'Unsupported manifest version: %s\n' "$manifest_version" >&2; exit 2; }
[[ "$archive_format" == 'custom' ]] || { printf 'Unsupported archive format: %s\n' "$archive_format" >&2; exit 2; }
[[ "$expected_bytes" =~ ^[0-9]+$ ]] || { printf 'Manifest bytes is invalid.\n' >&2; exit 2; }
[[ "$expected_sha256" =~ ^[a-f0-9]{64}$ ]] || { printf 'Manifest SHA-256 is invalid.\n' >&2; exit 2; }
[[ "$postgres_version" =~ ^[0-9]+$ && -n "$manifest_service" && -n "$created_at" ]] || { printf 'Manifest is incomplete.\n' >&2; exit 2; }

actual_bytes="$(wc -c <"$BACKUP_FILE" | tr -d ' ')"
[[ "$actual_bytes" == "$expected_bytes" ]] || { printf 'Backup size mismatch: manifest=%s actual=%s\n' "$expected_bytes" "$actual_bytes" >&2; exit 1; }
actual_sha256="$(sha256_file "$BACKUP_FILE")"
[[ "$actual_sha256" == "$expected_sha256" ]] || { printf 'Backup checksum mismatch.\n' >&2; exit 1; }
"${compose[@]}" exec -T "$service" pg_restore --list <"$BACKUP_FILE" >/dev/null
printf 'Verified backup: %s bytes, SHA-256 matches, PostgreSQL custom archive is readable.\n' "$actual_bytes"
