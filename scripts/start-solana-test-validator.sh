#!/usr/bin/env bash
set -euo pipefail

if ! command -v solana-test-validator >/dev/null 2>&1; then
  printf 'solana-test-validator is required. Install the pinned Solana CLI first.\n' >&2
  exit 1
fi

task_ledger="${LANDFALL_VALIDATOR_LEDGER:-$(mktemp -d /tmp/landfall-validator.XXXXXX)}"
printf 'Starting local validator with ledger: %s\n' "$task_ledger"

# macOS may otherwise create AppleDouble files such as ._genesis.bin inside the
# genesis archive, which Solana 1.18 rejects as an unexpected archive entry.
COPYFILE_DISABLE=1 exec solana-test-validator --ledger "$task_ledger" --reset "$@"
