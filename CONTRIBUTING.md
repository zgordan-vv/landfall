# Contributing to Landfall

Landfall is an early-stage open-source project. Small, focused changes with a
clear problem statement are easiest to review. For substantial product,
protocol, privacy, or architecture changes, open a discussion issue before
implementation.

## Development prerequisites

- Rust `1.98.0` with the components pinned by `rust-toolchain.toml`;
- Node.js `24.20.0` from `.node-version`;
- pnpm `11.25.0` from the root `package.json`;
- just `1.58.0` from `.just-version`;
- PostgreSQL and Docker will become required when the corresponding Phase 1
  tasks are implemented.

If your tool manager does not read `.just-version`, install the pinned command
runner with `cargo install just --version 1.58.0 --locked`.

Verify the pinned tools and fetch locked dependencies with:

```sh
just bootstrap
```

Run the complete local verification gate with:

```sh
just check
```

Run `just` without arguments to list focused formatting, linting, type-check,
test, build, development, and database recipes. Database recipes become
operational when Docker Compose is added in Phase 1 Task 8. `db-reset` is
destructive and requires the explicit `LANDFALL_CONFIRM_DB_RESET=YES` guard.

## Repository boundaries

- `landfall-protocol` and `packages/protocol-ts` own wire contracts and remain
  independent of application, database, UI, and provider code.
- Core domain behavior must not depend on HTTP, PostgreSQL, React, or Solana
  provider clients.
- The dashboard uses the public API client and never reads PostgreSQL directly.
- TypeScript workspace dependencies use `workspace:*` and public package
  exports; cross-package source imports and bypassing manifests with `paths`
  aliases are not allowed.
- Never add private keys, seed phrases, signed transaction bytes, bearer tokens,
  credential-bearing URLs, customer data, or real production identifiers to
  code, fixtures, tests, logs, screenshots, issues, or pull requests.

See [`crates/README.md`](crates/README.md),
[`packages/README.md`](packages/README.md), and
[`docs/threat-model.md`](docs/threat-model.md) for the complete boundaries.

## Change workflow

1. Create a focused branch from current `main`.
2. Add or update tests for behavior changes.
3. Update contracts and generated artifacts together when applicable.
4. Run the relevant checks locally.
5. Submit a pull request describing the problem, solution, verification, privacy
   impact, compatibility impact, and remaining limitations.

Do not combine unrelated refactoring with a functional change. Breaking event
or REST contract changes require an explicit versioning decision. Decisions
that are difficult to reverse or cross trust boundaries require an ADR based on
[`docs/adr/000-template.md`](docs/adr/000-template.md).

## Commit and review expectations

Descriptive commit messages are required; Conventional Commit prefixes are
welcome but not mandatory. Review checks correctness, tests, dependency
direction, public-contract compatibility, privacy, failure behavior,
documentation, and operational impact.

Report security issues through [`SECURITY.md`](SECURITY.md), not a public issue
or pull request.

## License

Unless explicitly marked otherwise, contributions intentionally submitted for
inclusion in Landfall are accepted under the
[Apache License 2.0](LICENSE), including its contribution terms. No separate
Contributor License Agreement is currently required.
