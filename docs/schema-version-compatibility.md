# Event schema version compatibility

## Purpose

Landfall selects an event contract before structural and semantic validation.
The selection is deliberately strict so a producer cannot assume that two
versions are compatible merely because they share a major number.

The canonical capability matrix lives in
[`schemas/events/v1/manifest.json`](../schemas/events/v1/manifest.json). Rust and
TypeScript expose small pure checks whose constants are tested against that
manifest and the same compatibility fixtures.

## Selection algorithm

Given the untrusted pair `(schema_version, event_type)`:

1. Look for an exact `schema_version` entry in `supported_versions`.
2. If none exists, return `LF_UNSUPPORTED_SCHEMA_VERSION`.
3. Look for the exact `event_type` in that version's advertised event list.
4. If none exists, return `LF_UNSUPPORTED_EVENT_TYPE`.
5. Only then select the bundled schema and continue with structural validation.

The version check intentionally has precedence. The result contains only a
stable code and never reflects the rejected strings. A later HTTP layer may add
the server's bounded supported-version list, but must not echo arbitrary input.

## Current matrix

| Declared version | Event type | Decision |
|---|---|---|
| `1.0` | one of the 15 registered v1.0 types | supported |
| `1.0` | unknown type | `LF_UNSUPPORTED_EVENT_TYPE` |
| `1.1`, `2.0`, older, or malformed | any type | `LF_UNSUPPORTED_SCHEMA_VERSION` |

`1.1` is not supported until its complete schema resources and capability row
are committed. A same-major version can contain an additive field or event that
an older collector does not know, so optimistic minor-version acceptance would
make validation and rollout nondeterministic.

## Change classification

A compatible minor release may add an optional bounded field, add a negotiated
event type, register a separately versioned extension, or clarify documentation
without changing accepted instances.

A new major is required when a change removes or renames a field/event, makes an
optional field required, changes a type/unit/meaning/privacy classification,
changes exhaustive closed-enum behavior, reinterprets absence/null/zero/error,
or structurally rejects previously valid persisted data.

Security and privacy policy versions remain independent. They may reject data
that structurally matches an older schema, but that is a policy rejection—not a
claim that the historical document never matched its wire version.

## Rollout and stored data

The rollout order is collector, CLI, then SDK. The collector must understand a
new version/event before a producer emits it in production. Capability/doctor
checks provide the deployment guard.

Raw events retain their original `schema_version` and are never relabeled. If a
future reducer needs a newer in-memory shape, a named deterministic upcaster
must record source and target versions and have golden tests. There is no
implicit coercion or current upcaster because `1.0` is the only registered
version.

## Executable enforcement

- Rust: `landfall_protocol::check_event_compatibility`.
- TypeScript: `checkEventCompatibility` from `@landfall/protocol`.
- Shared cases: [`fixtures/protocol/compatibility.json`](../fixtures/protocol/compatibility.json).
- Drift check: `just check-contracts` or `just check-protocol-compatibility`.

Adding a version requires, in one reviewed change, its schema resources,
manifest capability row, language constants/types, compatibility fixtures,
generation diff, documentation, and collector-before-producer rollout plan.
