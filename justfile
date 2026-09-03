set dotenv-load := false
set shell := ["bash", "-euo", "pipefail", "-c"]

# Show the available repository commands.
default:
    @just --list

# Verify pinned tools and install/fetch locked dependencies.
bootstrap:
    bash scripts/check-toolchain.sh
    cargo fetch --locked
    pnpm install --frozen-lockfile

# Format Rust, TypeScript, configuration, and root documentation files.
fmt: bootstrap
    cargo fmt --all
    pnpm run format

# Run architecture and static lint checks.
lint: bootstrap
    cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
    bash scripts/check-rust-dependency-direction.sh
    pnpm run lint

# Type-check both language workspaces without running tests.
typecheck: bootstrap
    cargo check --workspace --all-targets --all-features --locked
    pnpm run typecheck

# Run all currently registered unit and component tests.
test: bootstrap
    cargo test --workspace --all-targets --all-features --locked
    pnpm run test

# Run all currently registered integration-test targets.
test-integration: bootstrap
    cargo test --workspace --tests --all-features --locked
    pnpm run test:integration

# Build all Rust targets, TypeScript libraries, and the dashboard.
build: bootstrap
    cargo build --workspace --all-targets --all-features --locked
    pnpm run build

# Reproduce the complete local/CI verification gate from a clean checkout.
check: bootstrap _fmt-check lint typecheck test build

# Start the dashboard development server; backend runtime wiring arrives in Task 8.
dev: bootstrap
    @echo "The Rust server is still an empty skeleton; starting the dashboard only."
    pnpm run dev:dashboard

# Start PostgreSQL, wait for health, and verify version/storage/network invariants.
db-up:
    bash scripts/compose.sh up --detach --wait postgres
    bash scripts/check-postgres-compose.sh

# Destroy and recreate Landfall's PostgreSQL volume (requires LANDFALL_CONFIRM_DB_RESET=YES).
db-reset:
    @test "${LANDFALL_CONFIRM_DB_RESET:-}" = "YES" || { echo "Refusing destructive reset. Run: LANDFALL_CONFIRM_DB_RESET=YES just db-reset" >&2; exit 2; }
    bash scripts/compose.sh down --volumes --remove-orphans
    bash scripts/compose.sh up --detach --wait postgres
    bash scripts/check-postgres-compose.sh

[private]
_fmt-check:
    cargo fmt --all --check
    pnpm run format:check
