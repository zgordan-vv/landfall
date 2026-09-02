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
