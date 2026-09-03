#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

require_command() {
    local command_name="$1"

    if ! command -v "$command_name" >/dev/null 2>&1; then
        printf 'Required command is missing: %s\n' "$command_name" >&2
        return 1
    fi
}

require_exact_version() {
    local tool_name="$1"
    local expected="$2"
    local actual="$3"

    if [[ "$actual" != "$expected" ]]; then
        printf '%s version mismatch: expected %s, found %s\n' \
            "$tool_name" "$expected" "$actual" >&2
        return 1
    fi
}

require_command cargo
require_command just
require_command node
require_command pnpm
require_command rustc

expected_node="v$(tr -d '[:space:]' < .node-version)"
expected_just="just $(tr -d '[:space:]' < .just-version)"
expected_pnpm="$(node --input-type=module -e \
    'import manifest from "./package.json" with { type: "json" }; process.stdout.write(manifest.engines.pnpm);')"
expected_rust="$(sed -nE 's/^channel = "([0-9.]+)"$/\1/p' rust-toolchain.toml)"

require_exact_version "Node.js" "$expected_node" "$(node --version)"
require_exact_version "pnpm" "$expected_pnpm" "$(pnpm --version)"
require_exact_version "just" "$expected_just" "$(just --version)"
require_exact_version "Rust" "$expected_rust" "$(rustc --version | awk '{print $2}')"

printf 'Toolchain versions match the repository pins.\n'
