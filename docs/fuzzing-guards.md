# Malformed and high-cardinality fuzz guards

`fuzz_guards::validate_shape` is a cheap pre-parser safety gate for fuzz
corpora. It rejects JSON deeper than 32 levels, arrays over 1,000 items, and
objects over 1,000 keys. These limits complement compressed/decompressed body
limits and prevent pathological nesting or label cardinality from reaching
schema, database, or projection work. Corpus tests exercise each boundary and
ensure rejection is deterministic and panic-free.
