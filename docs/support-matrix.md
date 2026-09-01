# Landfall P0 Support Matrix

- **Status:** Frozen for P0 implementation
- **Version:** 1.0
- **Date:** 2026-09-01
- **Owners:** Landfall maintainers
- **Related decisions:** [ADR-004](adr/004-privacy-modes-and-signed-byte-fingerprints.md), [ADR-006](adr/006-solana-kit-first-adapter-and-compatibility-roadmap.md)

## 1. Purpose and authority

This document defines the combinations Landfall P0 may advertise as supported.
It is the operational source of truth for Node.js, the Solana JavaScript client,
PostgreSQL, Solana clusters, transaction versions, and privacy modes.

The ADRs own architecture and security invariants. This matrix narrows those
decisions to exact release lanes and required tests. If an example, PRD, system
design, package manifest, or implementation-plan sentence names a broader
version, this matrix wins for the P0 support claim.

Support is a tested contract, not “it probably works.” A tuple is supported only
when every axis in the tuple is supported and its required verification passes.
Landfall must never silently downgrade an unsupported tuple, reinterpret it as a
supported one, or report complete telemetry when capture is incomplete.

Normative terms `MUST`, `MUST NOT`, `SHOULD`, and `MAY` are used in their usual
requirements sense.

## 2. Status vocabulary

| Status | Meaning | Required product behavior |
|---|---|---|
| **Supported P0** | Part of the release contract and exercised by the required suite | Normal operation when all other axes are also supported |
| **Supported P0, external** | Product behavior is supported, but the external network is not deterministic or covered by an uptime promise | Opt-in tests, explicit network warning, bounded retries/budget |
| **Compatible, not supported** | A generic interface may work, but Landfall makes no P0 compatibility guarantee | Identify it honestly; never display “supported” |
| **Recognized, unsupported** | Landfall knows the value but has no implementation/support contract for it | Fail setup or mark capture incomplete with an actionable stable reason code |
| **Out of scope** | No P0 integration or compatibility claim | Reject configuration or direct the user to a supported alternative |

“Current patch” means the latest non-prerelease patch published for the frozen
major/minor lane and admitted by a reviewed dependency update. A newly published
upstream release does not become supported merely because a semver range matches.

## 3. Frozen P0 release lane

The first releasable end-to-end lane is:

| Axis | Frozen P0 value |
|---|---|
| Landfall | First P0 release; product version assigned later |
| SDK runtime | Node.js 24 LTS; release/test image pinned to 24.20.0 |
| Neutral TypeScript API | Supported on the Node.js lane without a Solana client dependency |
| Solana adapter | `@solana/kit` 8.2.0 exactly; no external Kit plugin is in the initial support claim |
| Database | PostgreSQL 18.6, pinned release image and immutable digest |
| Clusters | Local validator, Devnet, and Mainnet Beta under the limits in section 7 |
| Transaction formats | Legacy and version 0 (`v0`) |
| Privacy | `standard` by default or explicit `full` |

The implementation manifests, examples, lockfile, container image, generated
software bill of materials, CI logs, `landfall doctor`, and release notes MUST
agree with this row before P0 is released.

## 4. Node.js runtime

| Runtime | Status | Verification and behavior |
|---|---|---|
| Node.js `>=24.11.0 <25.0.0` | **Supported P0** | Compile/test at the LTS floor and at the pinned current 24.x patch; production documentation requires the current security patch |
| Node.js 24.20.0 | **P0 release pin** | Used by the release SDK example, benchmark reference environment, and containerized JS checks as of this matrix date |
| Node.js 22.x | **Recognized, unsupported** | Upstream LTS status does not add a second Landfall test lane; `doctor` returns `LF_UNSUPPORTED_NODE_VERSION` |
| Node.js 26.x Current | **Recognized, unsupported** | Reconsider only after it reaches LTS and the complete SDK/adapter suite passes |
| Node.js 20.x and older | **Out of scope** | EOL runtime; fail setup with upgrade guidance |
| Bun, Deno, browser, wallet extension, React Native | **Out of scope** | No P0 server-SDK claim; use the language-neutral ingestion protocol only if the caller implements the contract safely |

The package `engines.node` range MUST be `>=24.11.0 <25`. A warning is not enough
for an incompatible major: adapter construction and `doctor` MUST fail before
instrumentation is presented as active. Unsupported runtimes do not prevent a
customer application from running without Landfall.

## 5. Solana JavaScript client

| Client lane | Status | Verification and behavior |
|---|---|---|
| Neutral Landfall event API, no Solana client import | **Supported P0** on Node 24 | Schema golden fixtures, buffer/transport tests, and an end-to-end collector test |
| `@solana/kit` 8.2.0 | **Supported P0** | Exact development/example lock; compile, fixture, wrapper-contract, privacy, benchmark, and local-validator suites |
| Later Kit 8.2.x patch | **Recognized, unsupported until admitted** | Dependency update must pass the same suite and explicitly update this matrix before the peer range is widened |
| Kit 8.0.x, 8.1.x, 7.x, or another stable major | **Recognized, unsupported** | `LF_UNSUPPORTED_SOLANA_CLIENT_VERSION`; use 8.2.0 or the neutral API |
| Kit canary, RC, experimental, or git build | **Out of scope** | Never accepted by a production support lane |
| `@solana/web3.js` v3 | **Recognized, unsupported** | A time-boxed spike may follow P0; production requires its own adapter and matrix row |
| Legacy `@solana/web3.js` v1 | **Recognized, unsupported** | Add only after customer evidence justifies a permanent compatibility suite |
| `@solana/web3-compat` | **Out of scope** | Not a roadmap target; it does not make another client supported |

No external Kit plugin is required by the initial adapter contract. If
implementation introduces one, its package and exact version MUST be added to
the frozen lane before the adapter can be released. TypeScript structural
compatibility or a successful install is not runtime-support evidence.

Adapter initialization MUST validate the capabilities it uses. It must not
monkey-patch Kit, own customer retry/routing policy, or change the original
operation's returned value or error. The neutral SDK root MUST remain free of
Kit runtime imports.

## 6. PostgreSQL

| Server | Status | Verification and behavior |
|---|---|---|
| PostgreSQL 18.6 | **Supported P0 / release pin** | Real-database migration, repository, concurrency, lease-recovery, retention, report-storage, backup/restore, and performance tests |
| Later PostgreSQL 18 minor | **Recognized, unsupported until admitted** | Test release notes, migrations, jobs, restore, and benchmarks; then update image digest and this matrix |
| PostgreSQL 18.0–18.5 | **Recognized, unsupported** | Older fixes/security level than the release pin; readiness fails with `LF_UNSUPPORTED_POSTGRES_VERSION` |
| PostgreSQL 17 and earlier | **Recognized, unsupported** | No P0 CI or production claim even if SQL happens to work |
| PostgreSQL 19 prerelease | **Out of scope** | No beta/RC database in the P0 support contract |

At startup, the server MUST read `server_version_num`; an unsupported result
fails readiness before migrations, ingestion, projection, or background jobs
start. `landfall doctor` reports the server version without printing the
credential-bearing database URL.

A managed PostgreSQL service is compatible only when it reports the supported
server version and permits all migrations, transactions, row locks,
`FOR UPDATE SKIP LOCKED`, advisory-lock behavior if used, `BYTEA`, and backup/
restore procedures required by Landfall. Provider-specific proxies, failover,
limits, backups, and uptime remain operator/provider responsibilities; passing a
version check alone is not a blanket vendor certification.

The official release image MUST be pinned by immutable digest. PostgreSQL minor
updates are expected because upstream recommends the current minor, but each
update is a deliberate reviewed release rather than an untested floating tag.

## 7. Solana clusters and RPC endpoints

| Cluster/endpoint | Status | Intended P0 use and limits |
|---|---|---|
| Local `solana-test-validator` compatible validator | **Supported P0** | Deterministic development, CI integration, demo, success/failure fixtures; exact CLI/validator version is pinned when the harness is implemented |
| Devnet | **Supported P0, external** | Onboarding, opt-in integration, and controlled demo; ledger resets, rate limits, and newer software are expected external conditions |
| Mainnet Beta | **Supported P0, external** | Customer-controlled production observation and bounded read-only release smoke checks; transaction submission is never automatic in CI |
| Testnet | **Recognized, unsupported** | Intended upstream for validator/network stress; may be intermittent; `LF_UNSUPPORTED_SOLANA_CLUSTER` directs application developers to Devnet |
| Custom RPC exposing a known supported cluster | **Compatible, not independently certified** | Cluster identity, not endpoint hostname, controls the row; dedicated/private providers are allowed if the capability check passes |
| Custom genesis, fork, emulator, or modified validator | **Compatible, not supported** | Must be named `custom:<genesis-hash>`; no silent Mainnet/Devnet label and no claim that altered semantics are understood |

Environment setup MUST bind the configured cluster to an expected genesis
identity and verify it with standard JSON-RPC. A URL label such as `devnet` is
not identity. A mismatch fails readiness and observation; it must never mix data
from different clusters in one environment.

Public Solana RPC endpoints MAY be used for small demos and explicit checks but
MUST NOT be the documented production default. Mainnet production use requires
a customer-controlled dedicated/private endpoint, protected credentials,
bounded concurrency, and provider-aware rate limiting. Endpoint URLs, query
strings, and authorization material never enter events or diagnostics.

Required verification is intentionally different per cluster:

- local validator: mandatory deterministic CI, including restart/reset cases;
- Devnet: opt-in trusted-branch or scheduled smoke test with no uptime promise;
- Mainnet Beta: recorded fixtures plus bounded read-only checks against known
  public data; any funded experiment requires explicit operator action/budget;
- Testnet/custom genesis: negative or compatibility tests only.

## 8. Solana transaction versions

| Transaction/message format | Status | SDK and observer behavior |
|---|---|---|
| Legacy | **Supported P0** | Parse, serialize supported signed form, fingerprint exact signed bytes, submit/observe, project, diagnose, and report |
| Version 0 (`v0`) | **Supported P0** | Same contract as legacy, including address lookup-table fixtures; observer requests `maxSupportedTransactionVersion: 0` |
| Numeric version 1 or higher | **Recognized, unsupported** | Preserve the customer operation, emit `LF_UNSUPPORTED_TRANSACTION_VERSION`, mark Landfall capture incomplete, and do not parse or invent a fingerprint/result |
| Unknown or malformed version envelope | **Out of scope as telemetry input** | Reject/diagnose safely without crashing or dumping transaction bytes |

The positive suite MUST contain golden signed fixtures for legacy and `v0`,
including successful submission, identical-byte retry, replacement bytes,
simulation failure, timeout/later landing, address lookup tables, and expiration.
The negative suite MUST contain a synthetic higher-version envelope and prove:

1. the customer callback is invoked exactly once;
2. its original value or error identity is preserved;
3. no supported-version parser is used as a fallback;
4. no fingerprint, raw bytes, or fabricated lifecycle result is emitted;
5. the observer normalizes the higher-version RPC failure as incomplete evidence.

Durable nonce is not a transaction version. A legacy or `v0` durable-nonce
transaction may be identified and its observable lifecycle retained, but P0
does not provide recent-blockhash expiration diagnosis for it. The trace must
state `durable_nonce_detected` and the limitation instead of applying an
incorrect block-height rule.

Jito bundles and other multi-transaction submission semantics are also not
transaction versions. P0 may receive manual events for their individual
transactions, but it does not diagnose bundle-level ordering or atomicity.

## 9. Privacy modes

| Mode | Status | Central behavior |
|---|---|---|
| `standard` | **Supported P0 / default** | Minimized allowlisted telemetry, environment-keyed HMAC fingerprint, public signature and central observation; no account/program addresses by default |
| `full` | **Supported P0 / explicit opt-in** | `standard` plus only explicitly allowlisted public diagnostic identifiers and a persistent correlation warning |
| `strict` | **Recognized, unsupported in P0** | Startup/configuration fails with `LF_UNSUPPORTED_PRIVACY_MODE`; Landfall never silently falls back to `standard` |
| Unknown value | **Out of scope** | Configuration/schema rejection; no default substitution |

All P0 modes use `lf-hmac-sha256-v1` over exact serialized signed transaction
bytes with an environment key. Plain SHA-256, JSON, base64 text, unsigned
messages, and transaction-object serialization are not supported fingerprint
inputs. Raw signed transaction bytes are unsupported centrally in every mode.

The SDK and collector MUST both enforce the field matrix in ADR-004. A report
profile may be more restrictive than stored data but never more permissive.
Changing an environment mode affects new events; it neither relabels nor
silently deletes older data. Downgrading from `full` to `standard` therefore
requires the documented deletion/retention workflow if historical full-mode
identifiers must be removed.

## 10. Compatibility decision and failure contract

The complete tuple is evaluated in this order:

```text
Node runtime
  -> neutral SDK/schema
  -> optional Solana client adapter
  -> privacy mode
  -> cluster identity/RPC capabilities
  -> transaction version
  -> PostgreSQL server/readiness
```

Setup-time incompatibilities fail before Landfall claims instrumentation or
readiness. A transaction-version incompatibility discovered at runtime is
different: the customer's transaction remains customer-owned and continues
unchanged, while Landfall clearly reports incomplete capture.

Stable reason codes for P0 are:

- `LF_UNSUPPORTED_NODE_VERSION`;
- `LF_UNSUPPORTED_SOLANA_CLIENT_VERSION`;
- `LF_UNSUPPORTED_POSTGRES_VERSION`;
- `LF_UNSUPPORTED_SOLANA_CLUSTER`;
- `LF_CLUSTER_IDENTITY_MISMATCH`;
- `LF_UNSUPPORTED_TRANSACTION_VERSION`;
- `LF_UNSUPPORTED_PRIVACY_MODE`.

An incompatibility message MUST contain the detected safe value, the supported
lane, the affected capability, and one safe next action. It MUST NOT contain raw
signed bytes, transaction objects, secret-bearing URLs, tokens, fingerprint
keys, database credentials, arbitrary error objects, or stack traces.

`landfall doctor` MUST report:

- Landfall, schema, SDK, and adapter versions;
- Node.js version and whether the runtime lane matches;
- resolved Solana client/plugin tuple and required adapter capabilities;
- PostgreSQL server version and migration compatibility;
- configured cluster plus verified genesis identity, with sanitized route ID;
- supported transaction versions (`legacy`, `0`);
- active privacy mode and policy version;
- an overall `supported`, `unsupported`, or `incomplete` result with reason
  codes.

## 11. Release verification gates

| Gate | Minimum evidence before P0 release |
|---|---|
| Node | SDK/adapter compile and tests at 24.11.0 and 24.20.0; benchmark on the release pin |
| Kit | Exact 8.2.0 lock; typed compile fixture; wrapper identity/failure tests; unsupported-client negative fixture |
| PostgreSQL | Real 18.6 migrations, jobs, concurrency, restore, retention, report artifact, and capacity tests |
| Clusters | Mandatory local-validator integration; opt-in Devnet; recorded plus read-only Mainnet evidence; Testnet rejection |
| Transactions | Legacy and `v0` golden fixtures; higher-version fail-safe fixture; durable-nonce limitation fixture |
| Privacy | `standard`/`full` positive and negative field matrices; `strict` rejection; outbound/log scans for forbidden values |
| Tuple | `doctor` snapshot for one supported lane and each stable unsupported reason code |

No unit test with a mocked version string can substitute for the real-database,
real-runtime, exact-package, and local-validator gates above.

## 12. Change and deprecation policy

1. A dependency bot may propose a version, but cannot change support status.
2. A new Node major, Kit minor/major, PostgreSQL minor/major, cluster, transaction
   version, or privacy mode requires its relevant gates and a reviewed matrix
   edit in the same change.
3. A change to transaction interpretation, privacy, identity, or failure
   semantics also requires an ADR or an explicit superseding ADR update.
4. Removing a released lane requires release-note notice and a migration path:
   upgrade, remain on an explicitly maintained Landfall version, or use the
   neutral event protocol.
5. “May work” and “upstream supported” are never synonyms for “Landfall P0
   supported.”

## 13. Source baseline

Version and network facts were checked on 2026-09-01 against primary sources:

- [Node.js release schedule](https://nodejs.org/en/about/previous-releases) and
  [Node.js 24 LTS migration note](https://nodejs.org/en/blog/migrations/v22-to-v24);
- [PostgreSQL versioning policy](https://www.postgresql.org/support/versioning/);
- [`@solana/kit` package](https://www.npmjs.com/package/@solana/kit) and
  [official Kit repository](https://github.com/anza-xyz/kit);
- [Solana cluster purposes and public RPC limitations](https://solana.com/docs/references/clusters);
- [Solana `getTransaction` version contract](https://solana.com/docs/rpc/http/gettransaction).

These links explain upstream facts. Landfall's deliberately narrower support
promise is the matrix above.
