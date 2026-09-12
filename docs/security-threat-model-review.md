# Threat-model review against implementation

Reviewed 2026-09-12 against the current workspace. Existing controls are
mapped to concrete code rather than treated as design-only claims.

| Threat-model control | Implementation evidence | Result |
|---|---|---|
| Bounded ingestion work | `RequestBodyLimitLayer`, compressed limit, concurrency/rate limits in `landfall-server` | Covered |
| No credential fields in events | prohibited-key validation in server ingest | Covered |
| Constant-time token comparison | `auth::constant_time_eq` over SHA-256 digests | Covered |
| Export redaction and integrity | `export_safety`, `artifact_store` | Covered |
| Deletion auditability | `deletion::TraceTombstone` append-only ledger | Covered |
| Safe retention | `retention::plan_retention` rejects zero-day policy and supports dry-run | Covered |
| Object authorization and production CORS | full HTTP authorization middleware and production headers | Open — Phase 14.4/14.5 |
| TLS/reverse-proxy and dependency/SBOM gates | `tls-reverse-proxy.md`; release automation remains open | Partial — Phase 14.7 remains |

The open items are intentionally tracked as hardening work; this review does
not claim that a pure library model replaces runtime authorization or release
controls.
