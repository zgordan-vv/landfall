# Batch compression benchmark

Run `node scripts/benchmark-batch-compression.mjs`. It generates deterministic
JSON batches of 100 and 1,000 events, compresses them with gzip, and reports
raw bytes, compressed bytes, and the compressed/raw ratio. The payload shape is
representative of the wire envelope but contains no production data.

Record the Node version and compression implementation with results. This
benchmark measures serialization/compression cost only; network throughput and
server decompression belong to later benchmark steps.
