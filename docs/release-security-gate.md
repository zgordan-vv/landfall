# Release security gate

Run `scripts/release-security-gate.sh` before publishing. It requires both
lockfiles, verifies the container license label, runs the pinned Gitleaks
container scan, and checks approved Node dependency licenses. Missing Docker or
pnpm is reported as a skipped check for local development; CI should provision
both tools and treat skips as release failures. The repository currently uses
the exact Node/pnpm versions declared in `package.json`.
