# Malformed and high-cardinality fuzz guards

## Run

Run `cargo test -p landfall-server fuzz_guards` to execute the deterministic
boundary corpus. The HTTP layer still enforces compressed and decompressed byte
limits before this structural check.

## Contract

`fuzz_guards::validate_shape` rejects JSON deeper than 32 levels, arrays over
1,000 items, or objects over 1,000 keys. Rejection happens before schema,
database, or projection work. Tests cover each exact boundary and verify that
malformed input returns an error rather than panicking.

This is a structural guard, not a replacement for coverage-guided fuzzing;
libFuzzer/AFL integration remains a release-hardening follow-up.
