# Landfall Decimal-String Contract

- **Status:** Phase 2 Task 3 implementation baseline
- **Wire version:** `1.0`
- **Date:** 2026-09-08
- **Canonical schema:** [`values.schema.json`](../schemas/events/v1/1.0/common/values.schema.json)
- **Machine-readable bounds:** [`manifest.json`](../schemas/events/v1/manifest.json)
- **Related decision:** [ADR-005](adr/005-json-schema-event-contract-and-code-generation.md)

## 1. Why the wire format uses strings

JSON has one `number` syntax but does not define one universal runtime numeric
representation. JavaScript `number` uses IEEE-754 binary64 and represents only
integers from `-9007199254740991` through `9007199254740991` exactly. Solana
slots, block heights, lamport values, compute values, and nanosecond durations
are `u64`-shaped domains and can exceed that safe range.

Encoding these domains as JSON numbers would allow a valid value to change
during parse/serialize in TypeScript before Rust ever receives it. Landfall
therefore sends potentially large integers as canonical base-10 JSON strings.
This is an exact representation, not a display format.

## 2. Canonical lexical form

For `uint64_decimal`:

- the JSON value is a string, never a JSON number;
- only ASCII digits `0` through `9` are accepted;
- zero is exactly `"0"`;
- a non-zero value starts with `1` through `9`;
- the inclusive semantic range is `0..18446744073709551615`;
- the maximum lexical length is 20 characters.

For `int64_decimal`:

- positive values and zero follow the same rules;
- a negative value has exactly one leading `-` followed by a non-zero digit;
- `-0` is invalid;
- the inclusive semantic range is
  `-9223372036854775808..9223372036854775807`;
- the maximum lexical length, including `-`, is 20 characters.

Both domains reject leading/trailing whitespace, a leading `+`, leading zeroes,
decimal points, exponent notation, digit separators, hexadecimal notation,
Unicode digits, `NaN`, and infinity.

Examples:

| Value | Result | Reason |
|---|---|---|
| `"0"` | valid unsigned and signed | canonical zero |
| `"9007199254740992"` | valid | exactly one above JavaScript's safe range, still preserved |
| `"18446744073709551615"` | valid unsigned | maximum `u64` |
| `18446744073709551615` | invalid | JSON number instead of string |
| `"00"` | invalid | leading zero |
| `"+1"` | invalid | leading plus |
| `"-0"` | invalid | non-canonical zero |
| `"1e3"` | invalid | exponent notation |
| `"18446744073709551616"` | invalid unsigned | one above maximum `u64` |

## 3. Protocol domains

| Schema definition | Event fields | Unit/domain |
|---|---|---|
| `duration_ns_decimal` | `monotonic_ns`, `duration_ns`, `delay_ns`, `timeout_ns` | nanoseconds |
| `slot_decimal` | `slot`, `context_slot`, `min_context_slot` | Solana slot |
| `block_height_decimal` | `block_height`, `last_valid_block_height` | Solana block height |
| `lamports_decimal` | `fee_lamports` and future explicitly named lamport fields | lamports |
| `compute_units_decimal` | `units_consumed`, `compute_units_consumed` | compute units |
| `confirmation_count_decimal` | `confirmations` | confirmation count |

These aliases currently reuse `uint64_decimal`, but remain distinct schema
names so a future domain-specific bound or type can change deliberately rather
than silently changing every large integer.

Small bounded values remain JSON integers. P0 examples include
`required_signatures` (`1..64`), `instruction_index` (`0..255`), attempt/retry
sequences (`1..10000`), `max_retries` (`0..1000`), and custom program error
codes (`0..4294967295`). Every such field has a schema maximum below
JavaScript's safe-integer maximum.

## 4. Two-part validation

Portable JSON Schema patterns enforce type, canonical spelling, sign policy,
and digit count. A regular expression cannot cleanly express the exact `u64`
and `i64` endpoints without a fragile implementation-specific pattern.
Therefore typed semantic parsing performs the second check:

1. validate the canonical lexical form;
2. parse directly into the target integer type without floating point;
3. reject underflow or overflow;
4. serialize back in base 10 and require byte-for-byte equality with the input.

The collector performs both checks before persistence. SDK builders perform the
same checks before enqueueing an event, but collector validation remains
authoritative. No implementation may parse through JavaScript `number`, Rust
`f64`, or a database floating-point type.

## 5. Runtime and storage mapping

| Boundary | Unsigned 64-bit domain | Signed 64-bit domain |
|---|---|---|
| JSON wire/raw `JSONB` | canonical decimal string | canonical decimal string |
| TypeScript wire DTO | branded `string` | branded `string` |
| TypeScript arithmetic | `bigint` after checked parsing | `bigint` after checked parsing |
| Rust semantic value | checked `u64` newtype | checked `i64` newtype |
| PostgreSQL projected column | `NUMERIC(20,0)` plus range check | `BIGINT` |

PostgreSQL `BIGINT` is signed and cannot contain the upper half of `u64`, so
unsigned chain domains use `NUMERIC(20,0)` in typed projections. Raw events keep
the original string inside `JSONB`; selected envelope values such as
`monotonic_ns` use the same exact projected numeric representation.

TypeScript comparisons and arithmetic convert a validated wire string to
`bigint`. UI formatting may derive localized text from that value, but API
clients preserve the original decimal string. Lexicographic string ordering is
never numeric ordering unless values have first been parsed.

## 6. Compatibility rules

Changing an existing field between JSON string and JSON number is a breaking
wire change. Changing its unit, sign policy, or numeric domain is also breaking
unless a new optional field is introduced under an explicitly compatible minor
version.

Schema validation, future Rust/TypeScript conformance tests, fixtures, import,
export, report generation, and database projection must all use the bounds in
the version manifest. An older retained event is never rewritten merely because
a newer in-memory type or storage projection is introduced.
