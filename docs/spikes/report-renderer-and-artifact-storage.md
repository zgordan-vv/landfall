# Report Renderer and Artifact Storage Spike

**Status:** Completed

**Date:** 2026-09-01

**Decision:** Askama 0.16 for P0 HTML reports; application-gzip-compressed,
bounded PostgreSQL `BYTEA` for P0 HTML and JSON artifacts.

## 1. Purpose

The PRD requires a self-contained HTML report and a structured JSON report in
under 10 seconds for a cohort of 100,000 traces. The system design left two
implementation choices open:

1. Askama or MiniJinja for HTML rendering;
2. PostgreSQL `BYTEA` or a mounted artifact directory for the first release.

This spike resolves those choices with executable evidence. It is deliberately
small and is not the production report implementation.

## 2. Decision summary

### 2.1 Renderer

Use **Askama 0.16**.

Landfall owns and versions its report templates with the application; customers do
not supply templates at runtime. Askama therefore matches the product requirement:
it generates type-checked Rust from the template during compilation, removes
runtime template parsing and undefined-field failures, and produced the faster
render in this workload. MiniJinja remains a capable alternative if customer-owned
or runtime-editable templates become a validated requirement.

### 2.2 Artifact storage

Store P0 report artifacts as **gzip-compressed `BYTEA` in a separate PostgreSQL
artifact table**, with a configurable 10 MiB default limit applied independently to
the uncompressed and stored sizes.

This choice is primarily about correctness and operational simplicity, not the
single-machine timing result:

- report metadata, both artifacts, and the completed state can be published in one
  database transaction;
- rollback cannot leave committed metadata pointing to a missing file;
- the two-container topology requires only the existing PostgreSQL volume;
- database backup/restore contains metadata and artifacts from a consistent system;
- the 10 MiB cap prevents an accidental unbounded-blob design.

A mounted directory can atomically publish each individual file with
`write -> fsync -> rename -> fsync(directory)`, but it cannot atomically commit that
file together with PostgreSQL metadata. It adds orphan reconciliation, a second
persistent-volume/backup policy, path and permission hardening, and shared-storage
requirements if application roles later move to different hosts.

## 3. Candidate properties confirmed from upstream documentation

- [Askama](https://github.com/askama-rs/askama) generates type-safe Rust code for
  templates at compile time and supports automatic HTML escaping.
- [MiniJinja](https://docs.rs/minijinja/2.24.0/minijinja/) loads templates into a
  runtime environment, compiles them to bytecode, renders Serde-compatible values,
  and supports automatic HTML escaping.
- PostgreSQL [`BYTEA`](https://www.postgresql.org/docs/18/datatype-binary.html) is
  the built-in variable-length binary type.
- PostgreSQL [TOAST](https://www.postgresql.org/docs/18/storage-toast.html)
  transparently compresses and/or moves large variable-length values out of the
  main table row. Application-gzip data will normally gain little from a second
  compression pass, but it may still be stored out of line.

The renderer decision does not rely only on the performance numbers below. The
compile-time versus runtime template lifecycle is the stronger product-fit signal.

## 4. Reproducible experiment

The executable spike is in
[`spikes/report-renderer-storage`](../../spikes/report-renderer-storage/README.md).
The captured machine-readable output is
[`macos-m2-postgresql14.json`](../../spikes/report-renderer-storage/results/macos-m2-postgresql14.json).

### 4.1 Workload

The program deterministically:

1. processes 100,000 synthetic traces into cohort metrics and lifecycle buckets;
2. includes 10,000 explicitly selected trace timelines in the portable report;
3. renders the same HTML template with Askama and MiniJinja;
4. serializes JSON from the same typed report model;
5. checks that untrusted HTML is escaped and that the HTML has no external assets;
6. calculates SHA-256 over canonical uncompressed bytes;
7. compresses HTML and JSON with application-level gzip;
8. rejects either an uncompressed or stored artifact over 10 MiB;
9. stores report metadata and both artifacts in one PostgreSQL transaction;
10. writes the same artifacts to a directory with durable atomic renames;
11. reads, decompresses, size-checks, and verifies every artifact byte for byte.

Ten thousand selected timelines deliberately make rendering and artifact size
non-trivial. Metrics still cover all 100,000 input traces. A production request may
select fewer timelines.

### 4.2 Reference environment

- MacBook Air, Apple M2, 8 cores, 8 GiB RAM;
- macOS arm64;
- Rust 1.89.0, release profile with thin LTO;
- Askama 0.16.0;
- MiniJinja 2.24.0 with `speedups` and strict undefined values;
- isolated PostgreSQL 14.17 cluster with default durability settings;
- five warmed render/storage samples.

PostgreSQL 14 was the locally available server. PostgreSQL 18 documentation was
used for the target design. The spike validates the ordinary `BYTEA`, constraint,
transaction, rollback, and readback behavior used by the design; the P0 support
matrix must still run the database suite against the exact supported PostgreSQL
version.

## 5. Results

### 5.1 End-to-end selected pipeline

| Measurement | Result |
|---|---:|
| Process 100,000 traces, render Askama HTML + JSON, SHA-256 + gzip | 71.47 ms |
| PRD target | under 10,000 ms |
| Target met | yes |

This is large headroom, not an SLO claim. The spike uses synthetic in-memory input
and does not include production database queries or the final prohibited-field
scanner.

### 5.2 Renderer comparison

| Measurement | Askama 0.16 | MiniJinja 2.24 |
|---|---:|---:|
| Median warm render | 4.10 ms | 9.58 ms |
| Runtime parse/setup | none | 0.38 ms |
| HTML bytes | 4,258,237 | 4,258,240 |
| Escaping probe | passed | passed |
| No external assets | passed | passed |

Askama was about 2.3 times faster in this narrow workload. The three-byte output
difference comes from renderer-specific but equivalent escaping/newline behavior;
both outputs passed the semantic portability and escaping checks.

### 5.3 Artifact size

| Format | Uncompressed | Gzip stored | Stored/raw |
|---|---:|---:|---:|
| HTML | 4,258,237 bytes | 168,496 bytes | 3.96% |
| JSON | 3,875,957 bytes | 163,903 bytes | 4.23% |

The synthetic rows repeat more than real incident evidence, so these compression
ratios must not be used for production capacity estimates. The important result is
that both raw and stored values were below independently enforced 10 MiB caps.

### 5.4 Storage comparison

Each sample durably published the same gzip HTML and JSON artifacts.

| Measurement | PostgreSQL `BYTEA` | Mounted directory |
|---|---:|---:|
| Median publish | 4.17 ms | 10.21 ms |
| Last-report readback | 8.43 ms | 6.09 ms |
| Byte/gzip/checksum round-trip | passed | passed |
| Atomic individual artifact | yes | yes, by rename |
| Atomic metadata + both artifacts | yes | no |
| 10 MiB database constraint | passed | application-only |

The latency difference is not generalized beyond this machine. Both choices are
comfortably fast for a report worker. PostgreSQL wins because it preserves the
publication invariant and the P0 deployment/backup model with fewer failure modes.

## 6. Production implementation contract

### 6.1 Rendering

1. Build one stable, typed `ReportModel` from a frozen cohort watermark.
2. Serialize JSON from that model with Serde.
3. Render self-contained HTML from that model with Askama.
4. Keep templates inside the compiled application and compile-fail on invalid
   field access.
5. Retain default HTML escaping. Any future `safe`/unescaped fragment requires a
   narrowly typed wrapper and a security test.
6. Render through a bounded/counting writer in production so the process stops
   before allocating an artifact larger than the raw cap.
7. Run export redaction and the prohibited-secret scan on uncompressed content
   before compression and publication.

### 6.2 Artifact envelope

For each `html` or `json` artifact, persist:

| Field | Meaning |
|---|---|
| `format` | `html` or `json` |
| `content_type` | `text/html; charset=utf-8` or `application/json` |
| `content_encoding` | `gzip` |
| `uncompressed_size` | Exact canonical byte length, maximum 10 MiB by default |
| `stored_size` | Exact gzip byte length, maximum 10 MiB by default |
| `sha256` | 32-byte digest of canonical uncompressed bytes |
| `bytes` | Exact gzip bytes in PostgreSQL `BYTEA` |

The checksum covers uncompressed canonical bytes so integrity does not depend on a
gzip encoder version or compression level. The database checks digest length,
declared stored size against `octet_length(bytes)`, allowed formats/encodings, and
both declared size caps. The application calculates sizes; it does not accept them
from an API client.

Decompression must use a bounded reader and reject output above
`uncompressed_size` or the configured cap before checksum comparison. The report
download response uses stored `Content-Type` and `Content-Encoding: gzip`; the CLI
may decompress when exporting an offline file.

### 6.3 Publication transaction

Within one successful PostgreSQL transaction:

1. lock or verify the pending report row and stable report version;
2. insert the HTML and JSON artifact rows;
3. update the report to `completed` with its completion timestamp;
4. commit.

If any insert or state transition fails, rollback the entire publication. A
separate transaction may then mark the report `failed`; it must never expose a
`completed` report with one missing artifact. Idempotent retries use the stable
report ID plus format as the artifact key.

### 6.4 Initial schema shape

```sql
CREATE TABLE reporting.report_artifacts (
    report_id UUID NOT NULL REFERENCES reporting.reports(id) ON DELETE CASCADE,
    format TEXT NOT NULL CHECK (format IN ('html', 'json')),
    content_type TEXT NOT NULL,
    content_encoding TEXT NOT NULL CHECK (content_encoding = 'gzip'),
    uncompressed_size BIGINT NOT NULL
        CHECK (uncompressed_size BETWEEN 0 AND 10485760),
    stored_size BIGINT NOT NULL
        CHECK (stored_size BETWEEN 0 AND 10485760),
    sha256 BYTEA NOT NULL CHECK (octet_length(sha256) = 32),
    bytes BYTEA NOT NULL,
    PRIMARY KEY (report_id, format),
    CHECK (stored_size = octet_length(bytes))
);
```

The production migration should obtain the byte cap from configuration at the
application boundary; a migration-time database constraint provides defense in
depth for the initial default. A later configurable value above 10 MiB requires a
coordinated constraint migration or a fixed hard database ceiling plus a lower
runtime setting.

## 7. When to replace `BYTEA`

Revisit storage and prefer object storage when measured evidence shows one or more
of these conditions:

- the required artifact limit materially exceeds 10 MiB;
- retained reports cause unacceptable database backup, replication, vacuum, or
  restore cost;
- hosted multi-instance deployment needs direct streaming, range requests, CDN
  delivery, or independent artifact lifecycle policies;
- report volume exceeds the P0 2–5 GiB storage budget;
- customer requirements demand separate encryption keys or retention for exports.

At that point store an immutable object key, checksum, size, and state in
PostgreSQL. Use staged upload followed by an explicit publish/finalize protocol and
orphan reconciliation; do not replace the current atomic invariant with an
uncontrolled database/object-store dual write.

## 8. Limitations

- Synthetic data does not reproduce production query latency, cardinality, or
  compression entropy.
- Five warmed samples on one laptop are selection evidence, not capacity evidence.
- PostgreSQL 14.17 was used as a compatibility proxy, not the final supported
  database version.
- No concurrent report workers, retention, replication, backup, or vacuum load was
  simulated.
- Renderer build-time and binary-size effects were not measured.
- The final secret scanner and streaming HTTP response were not implemented here.

These limitations do not block the P0 selection. Phase 15 must rerun an integrated
benchmark with the supported PostgreSQL version, production queries, concurrent
workers, the prohibited-field scan, and the published reference-hardware profile.
