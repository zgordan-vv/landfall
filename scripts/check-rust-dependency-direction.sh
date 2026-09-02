#!/usr/bin/env bash

set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repository_root"

cargo_command="${CARGO:-cargo}"

direct_internal_dependencies() {
    local package="$1"

    "$cargo_command" tree \
        --package "$package" \
        --edges all \
        --depth 1 \
        --prefix none \
        --format '{p}' \
        | awk 'NR > 1 && $1 ~ /^landfall-/ { print $1 }' \
        | LC_ALL=C sort -u
}

assert_direct_internal_dependencies() {
    local package="$1"
    local expected="$2"
    local actual

    actual="$(direct_internal_dependencies "$package")"
    if [[ "$actual" != "$expected" ]]; then
        printf 'Invalid direct Landfall dependencies for %s.\n' "$package" >&2
        printf 'Expected:\n%s\n' "${expected:-<none>}" >&2
        printf 'Actual:\n%s\n' "${actual:-<none>}" >&2
        return 1
    fi
}

assert_dependency_tree_excludes() {
    local package="$1"
    local forbidden_pattern="$2"
    local violations

    violations="$($cargo_command tree \
        --package "$package" \
        --edges all \
        --prefix none \
        --format '{p}' \
        | awk -v pattern="$forbidden_pattern" '$1 ~ pattern { print $1 }' \
        | LC_ALL=C sort -u)"

    if [[ -n "$violations" ]]; then
        printf 'Forbidden dependencies are reachable from %s:\n%s\n' \
            "$package" "$violations" >&2
        return 1
    fi
}

assert_direct_internal_dependencies "landfall-protocol" ""
assert_direct_internal_dependencies "landfall-core" "landfall-protocol"
assert_direct_internal_dependencies "landfall-storage" $'landfall-core\nlandfall-protocol'
assert_direct_internal_dependencies "landfall-observer" $'landfall-core\nlandfall-protocol'
assert_direct_internal_dependencies "landfall-report" $'landfall-core\nlandfall-protocol'
assert_direct_internal_dependencies "landfall-server" $'landfall-core\nlandfall-observer\nlandfall-protocol\nlandfall-report\nlandfall-storage'
assert_direct_internal_dependencies "landfall-cli" "landfall-server"

assert_dependency_tree_excludes \
    "landfall-protocol" \
    '^(agave-client|askama|axum|deadpool-postgres|diesel|hyper|landfall-cli|landfall-core|landfall-observer|landfall-report|landfall-server|landfall-storage|minijinja|postgres|reqwest|sea-orm|solana-client|solana-pubsub-client|solana-rpc-client|solana-rpc-client-api|sqlx|tokio-postgres|tower|tower-http)$'
assert_dependency_tree_excludes \
    "landfall-core" \
    '^(agave-client|askama|axum|deadpool-postgres|diesel|hyper|landfall-cli|landfall-observer|landfall-report|landfall-server|landfall-storage|minijinja|postgres|reqwest|sea-orm|solana-client|solana-pubsub-client|solana-rpc-client|solana-rpc-client-api|sqlx|tokio-postgres|tower|tower-http)$'

printf 'Rust crate dependency direction is valid.\n'
