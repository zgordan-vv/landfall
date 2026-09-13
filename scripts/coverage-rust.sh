#!/usr/bin/env bash

set -euo pipefail

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
    printf 'cargo-llvm-cov is required. Install it with: cargo install cargo-llvm-cov --locked\n' >&2
    exit 127
fi

mkdir -p coverage/rust
cargo llvm-cov \
    --workspace \
    --all-targets \
    --all-features \
    --locked \
    --lcov \
    --output-path coverage/rust/lcov.info
cargo llvm-cov report
