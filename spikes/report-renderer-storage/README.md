# Report renderer and artifact-storage spike

This standalone spike measures the two decisions left open for Landfall P0:

- Askama versus MiniJinja for the self-contained HTML report;
- bounded gzip artifacts in PostgreSQL `BYTEA` versus a mounted directory.

It deterministically processes 100,000 synthetic traces, embeds 10,000 selected
timelines, renders HTML and JSON from the same model, enforces the 10 MiB raw and
stored caps, computes SHA-256 over the uncompressed canonical bytes, and verifies
gzip readback. The HTML uses inline CSS and no server or internet assets.

The PostgreSQL test stores report metadata and both artifacts in one transaction,
verifies byte-for-byte readback, proves rollback removes the metadata and artifact
together, and proves the database constraint rejects an artifact over the cap. The
directory test uses `write -> fsync -> rename -> fsync(directory)` for each file.

Run the unit tests:

```bash
cargo test --manifest-path spikes/report-renderer-storage/Cargo.toml
```

Run only the renderer/artifact pipeline:

```bash
cargo run --release --manifest-path spikes/report-renderer-storage/Cargo.toml -- \
  --traces 100000 \
  --selected 10000 \
  --iterations 5
```

For the complete storage comparison, pass an isolated PostgreSQL connection and a
fresh temporary artifact directory:

```bash
cargo run --release --manifest-path spikes/report-renderer-storage/Cargo.toml -- \
  --traces 100000 \
  --selected 10000 \
  --iterations 5 \
  --postgres-url 'host=/tmp/landfall-spike/socket user=landfall_spike dbname=postgres' \
  --artifact-dir /tmp/landfall-spike/artifacts
```

The command prints machine-readable JSON. The captured reference result lives in
`results/macos-m2-postgresql14.json`; the reviewed result and decision live in
`docs/spikes/report-renderer-and-artifact-storage.md`.
