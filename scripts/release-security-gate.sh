#!/usr/bin/env bash

set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root_dir"

[[ -f Cargo.lock ]] || { echo "Cargo.lock is required" >&2; exit 1; }
[[ -f pnpm-lock.yaml ]] || { echo "pnpm-lock.yaml is required" >&2; exit 1; }
[[ -f Dockerfile ]] || { echo "Dockerfile is required" >&2; exit 1; }
grep -q 'org.opencontainers.image.licenses' Dockerfile || { echo "Dockerfile license label is required" >&2; exit 1; }

echo "lockfiles and container metadata: ok"
if command -v docker >/dev/null 2>&1; then
  scripts/check-secrets.sh
else
  echo "secret scan: skipped (Docker unavailable)" >&2
fi
if command -v pnpm >/dev/null 2>&1; then
  node scripts/check-node-licenses.mjs
else
  echo "Node license scan: skipped (pnpm unavailable)" >&2
fi
echo "release security gate passed"
