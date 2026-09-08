#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

# Canonical event schemas now exist. Their registry, local references, closed
# event union, and security-sensitive property allowlist are checked without
# network resolution. Generated artifacts remain forbidden until their
# deterministic generators are registered in later tasks.
node scripts/check-event-schemas.mjs

unregistered_generated_paths=(
    openapi
    packages/api-client/src/generated
    packages/protocol-ts/src/generated
)

for contract_path in "${unregistered_generated_paths[@]}"; do
    if [[ -e "$contract_path" ]]; then
        printf 'Contract path %s exists, but no generator is registered in the drift check.\n' \
            "$contract_path" >&2
        exit 1
    fi
done

printf 'Canonical event schemas are registered; no unregistered generated contracts exist.\n'
