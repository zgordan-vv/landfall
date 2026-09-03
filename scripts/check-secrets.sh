#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! command -v docker >/dev/null 2>&1; then
    printf 'Docker is required for the pinned Gitleaks scan.\n' >&2
    exit 127
fi

gitleaks_image='ghcr.io/gitleaks/gitleaks:v8.30.1@sha256:c00b6bd0aeb3071cbcb79009cb16a60dd9e0a7c60e2be9ab65d25e6bc8abbb7f'

docker run --rm \
    --volume "$repository_root:/repository:ro" \
    --workdir /repository \
    "$gitleaks_image" \
    git --no-banner --no-color --redact --verbose /repository
