#!/usr/bin/env bash
set -euo pipefail

version="${LANDFALL_RELEASE_VERSION:-$(git describe --tags --always --dirty)}"
[[ "$version" != *-dirty ]] || { echo 'Refusing to archive a dirty worktree.' >&2; exit 2; }
output_dir="${LANDFALL_RELEASE_DIR:-dist/release}"
image_reference="${LANDFALL_RELEASE_IMAGE:-landfall-server:local}"
mkdir -p "$output_dir"
archive="$output_dir/landfall-${version}.tar.gz"
git archive --format=tar.gz --prefix="landfall-${version}/" HEAD >"$archive"

shasum -a 256 "$archive" >"$archive.sha256"
{
  printf 'version=%s\ncommit=%s\narchive=%s\n' "$version" "$(git rev-parse HEAD)" "$(basename "$archive")"
  printf 'dockerfile_sha256=%s\n' "$(shasum -a 256 Dockerfile | awk '{print $1}')"
  printf 'image_reference=%s\n' "$image_reference"
  if docker image inspect "$image_reference" >/dev/null 2>&1; then
    printf 'image_id=%s\n' "$(docker image inspect --format '{{.Id}}' "$image_reference")"
  else
    printf 'image_id=not-built\n'
  fi
} >"$output_dir/landfall-${version}.provenance"
printf 'Release archive: %s\nChecksum: %s\nProvenance: %s\n' "$archive" "$archive.sha256" "$output_dir/landfall-${version}.provenance"
