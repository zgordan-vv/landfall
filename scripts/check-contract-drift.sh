#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

# Verify canonical schemas and deterministically regenerate the committed
# TypeScript representation without network resolution.
node scripts/check-event-schemas.mjs
node scripts/generate-protocol-ts.mjs --check
node scripts/check-protocol-fixtures.mjs
node scripts/check-protocol-compatibility.mjs
node scripts/check-protocol-privacy-fixtures.mjs
node scripts/check-protocol-privacy-classification.mjs

unregistered_generated_paths=(
    openapi
    packages/api-client/src/generated
)

for contract_path in "${unregistered_generated_paths[@]}"; do
    if [[ -e "$contract_path" ]]; then
        printf 'Contract path %s exists, but no generator is registered in the drift check.\n' \
            "$contract_path" >&2
        exit 1
    fi
done

printf 'Canonical schemas, generated types, fixtures, compatibility, and privacy rules are current and fully classified.\n'
