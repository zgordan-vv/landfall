# Rust crate boundaries

The Landfall backend is one deployable modular monolith. Separate crates make
the source boundaries explicit without turning the P0 deployment into a set of
network services.

## Dependency direction

```text
landfall-protocol
        ^
        |
landfall-core
        ^
        |
        +----------------+----------------+
        |                |                |
landfall-storage  landfall-observer  landfall-report
        ^                ^                ^
        +----------------+----------------+
                         |
                  landfall-server
                         ^
                         |
                    landfall-cli
```

Arrows point from a consumer to the dependency it may use. In concrete terms:

- `landfall-protocol` owns versioned wire vocabulary and depends on no other
  Landfall crate;
- `landfall-core` owns deterministic domain behavior and depends only on
  `landfall-protocol` inside the workspace;
- `landfall-storage`, `landfall-observer`, and `landfall-report` are sibling
  adapters that depend inward on `landfall-core` and `landfall-protocol`, never
  on one another;
- `landfall-server` is the composition root and may wire the core and all three
  adapters;
- `landfall-cli` uses the server's public service surface and does not reach
  around it into storage or provider adapters.

The neutral protocol and domain core must not acquire web, database, report
template, or Solana RPC implementation dependencies. This keeps lifecycle and
diagnostic rules testable without PostgreSQL, HTTP, or an external network.

## Enforcement

Run:

```shell
./scripts/check-rust-dependency-direction.sh
```

The command checks every crate's direct internal dependencies and also scans
the complete dependency trees of `landfall-protocol` and `landfall-core` for
forbidden infrastructure packages. Cargo itself rejects dependency cycles.

When a new crate or architectural exception is needed, update the architecture
decision first, then change the expected graph in
`scripts/check-rust-dependency-direction.sh` in the same reviewed commit.

## Shared workspace policy

The root `Cargo.toml` is authoritative for:

- Rust Edition 2024 and minimum supported Rust `1.98.0`;
- internal crate paths with default features disabled;
- Rust, Clippy, and rustdoc lints inherited by every crate;
- the production release profile.

Every crate declares an empty `default` feature set. New functionality must
remain unconditional or be placed behind a specifically named opt-in feature;
depending on another workspace crate never activates its future defaults
implicitly.

The workspace forbids unsafe Rust and ignored `must_use` results. Clippy warns
on its `all` and `pedantic` groups and rejects debugging macros, `unwrap`,
`expect`, explicit `panic!`, `todo!`, and `unimplemented!` branches. Missing
public documentation and broken rustdoc links are caught
before release. A narrow lint exception is allowed only next to the relevant
code with a reason.

The release profile uses thin link-time optimization and one code-generation
unit for optimized binaries. Integer overflow checks stay enabled because
Landfall evaluates balances, fees, block heights, timestamps, and counters.
Panics abort the process instead of attempting to continue from an unexpected
invariant violation; durable state and job recovery belong to PostgreSQL, not
process memory. Debug information is stripped while ordinary symbols remain
available for operational diagnosis.
