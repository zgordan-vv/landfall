#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

# Phase 1 intentionally has no generated public contracts. This list makes that
# state explicit: introducing even one canonical or generated contract path must
# also replace this guard with deterministic generation and byte comparison.
contract_paths=(
    openapi
    schemas
    packages/api-client/src/generated
    packages/protocol-ts/src/generated
)

for contract_path in "${contract_paths[@]}"; do
    if [[ -e "$contract_path" ]]; then
        printf 'Contract path %s exists, but no generator is registered in the drift check.\n' \
            "$contract_path" >&2
        exit 1
    fi
done

printf 'No schema or OpenAPI artifacts exist yet; the Phase 1 absence guard is intact.\n'
