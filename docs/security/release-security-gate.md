# Release security gate

## Run

From the repository root run `scripts/release-security-gate.sh`. A successful
run prints checks for lockfiles/container metadata, Gitleaks, Node licenses,
and finally `release security gate passed`.

## Fail-closed expectations

The script requires `Cargo.lock`, `pnpm-lock.yaml`, and the Dockerfile license
label. In CI, Docker and pnpm must be installed; local skips are visible and
must not be accepted as a release result. The Node license check also requires
the exact runtime declared in `package.json` (Node 24.20.0 / pnpm 11.25.0).
