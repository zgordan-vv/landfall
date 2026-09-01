# ADR-008: Code-first OpenAPI with Utoipa

- **Status:** Accepted
- **Date:** 2026-09-01
- **Decision owners:** Landfall maintainers
- **Related requirements:** FR-INGEST-001 through FR-INGEST-003, FR-OBS-001 through FR-OBS-003, FR-REPORT-001, FR-REPORT-002, FR-ADMIN-001, API design and security requirements
- **Related ADRs:** [ADR-003](003-business-action-trace-attempt-event-and-alias-identifiers.md), [ADR-005](005-json-schema-event-contract-and-code-generation.md), [ADR-006](006-solana-kit-first-adapter-and-compatibility-roadmap.md)
- **Supersedes:** None
- **Superseded by:** None

## Context

Landfall exposes one REST/JSON API to its TypeScript dashboard, Rust CLI,
instrumentation SDK, operators, and possible future integrations. The API
includes event ingestion, trace queries, metrics, recommendation disposition,
reports, deletion, configuration, and health endpoints. Request/response types,
HTTP status codes, authentication, headers, pagination, privacy restrictions,
and error envelopes are all part of the product contract.

The server is implemented with Axum. Maintaining Axum routes, Rust Serde types,
a handwritten OpenAPI document, and TypeScript client types independently would
create several drift paths. A route could exist but be absent from documentation;
an annotation could advertise a status the handler never returns; or a generated
client could send a field that Serde rejects.

Code-first generation reduces that duplication but does not eliminate it.
OpenAPI annotations are still metadata, middleware can add responses that a
handler signature does not show, and runtime business validation is stricter
than a type declaration. The design therefore needs route-linked generation,
checked artifacts, contract tests, semantic version rules, and explicit
ownership for every part of the wire contract.

Landfall also has two different public contract families:

1. Lifecycle events are produced outside the server and are governed by the
   language-neutral JSON Schema contract accepted in ADR-005.
2. REST resources are implemented by the Rust server and can be described from
   the same Rust DTOs and handlers that execute them.

OpenAPI must document the ingestion endpoint without redefining the event union
in Utoipa. Making OpenAPI the owner of event fields would reverse ADR-005 and
could make the SDK, collector validator, and REST documentation disagree.

## Decision drivers

- Keep Axum route registration and OpenAPI path registration as close to one
  operation as the toolchain allows.
- Use the same Rust wire DTOs for Serde and REST schema generation.
- Preserve JSON Schema as the sole authority for lifecycle event documents.
- Produce deterministic, reviewable, offline release artifacts.
- Generate strict TypeScript types for dashboard and integration use without a
  large generated runtime client.
- Document every response status, error envelope, authentication requirement,
  significant header, content type, and bounded parameter.
- Make breaking API changes visible before merge and provide a versioning rule.
- Avoid relying on OpenAPI documentation as runtime validation.
- Prevent examples, server URLs, descriptions, and generated code from leaking
  tokens, RPC credentials, signatures, or customer data.
- Support the self-hosted product without a CDN or hosted schema registry.
- Keep the implementation understandable enough to explain in a portfolio and
  maintainable by a small team.

## Options considered

### Option A — Code-first Rust with Utoipa and route-linked Axum integration

Rust handlers and REST DTOs carry Utoipa metadata. `utoipa-axum` composes Axum
routes and OpenAPI paths together. A deterministic exporter creates a checked-in
OpenAPI document, which is validated and used to generate TypeScript types.

This fits the selected Rust/Axum stack and minimizes path/type duplication. It
still requires annotations, snapshot review, response tests, and special
composition for the independently owned event schema.

### Option B — Design-first OpenAPI and generated Rust server interfaces

A handwritten OpenAPI document could become canonical, with Rust server stubs
and TypeScript clients generated from it. This is attractive for a large public
API governed by a dedicated API-design team. For P0, generated Axum server code
would add an abstraction layer around domain-specific validation and middleware,
and developers would still need to reconcile the independent event schema.

### Option C — Handwritten OpenAPI alongside handwritten Axum

This provides full control over the document and no macro dependency. Every
route, parameter, DTO, rename, response, and security rule would be duplicated.
Snapshot tests can detect textual changes but cannot prove the handwritten
document matches runtime behavior.

### Option D — Generate OpenAPI only from JSON Schema

JSON Schema can describe JSON values but does not own HTTP methods, paths,
headers, authentication, status codes, request bodies, or response content. It
is correct for the event protocol and insufficient as the only REST source.

### Option E — No machine-readable REST contract

Human Markdown and manually typed frontend calls would reduce initial tooling.
It would forfeit generated client types, automated breaking-change detection,
interactive exploration, and a strong portfolio demonstration of cross-language
contract discipline.

### Option F — Full third-party TypeScript SDK generation

A generator such as Hey API can emit endpoint functions and a Fetch runtime in
addition to types. This offers convenience but introduces a larger generated
surface and runtime dependency. The current tool is pre-1.0 and explicitly asks
users to pin exact releases. P0 instead generates types and owns a small Fetch
transport; full SDK generation can be reconsidered if that transport becomes
costlier than expected.

## Decision

Landfall REST APIs will use code-first OpenAPI generated from Rust with
`utoipa` 5.5 and `utoipa-axum` 0.2 on Axum 0.8. Exact crate versions are locked
in `Cargo.lock` and updated deliberately. The emitted contract is OpenAPI 3.1.0,
the version serialized by the selected Utoipa line.

Every JSON REST endpoint is registered through `OpenApiRouter` (or an explicitly
reviewed wrapper around it), so the runtime router and OpenAPI path are composed
together. Direct `axum::Router::route` calls are prohibited for documented API
routes. Static dashboard assets and other deliberately non-OpenAPI services are
kept on a small explicit allowlist and tested separately.

Rust REST DTOs plus Serde behavior are authoritative for REST serialization;
Utoipa annotations are authoritative documentation metadata for paths,
parameters, responses, headers, and security. The generated checked-in OpenAPI
snapshot is the reviewed release contract and the sole input to downstream
TypeScript generation. It is derived and must never be edited manually.

The lifecycle event schema remains authoritative under ADR-005. During export,
the canonical bundled event JSON Schema is mechanically inserted into the
ingestion request component and referenced by the route. No Utoipa struct or
annotation may independently restate the event union.

P0 will generate TypeScript `paths` and component types with exactly pinned
`openapi-typescript` 7.13.0. A small Landfall-owned client uses the native Fetch
API and the generated types. P0 will not depend on `openapi-fetch`; its
maintainers announced maintenance mode for non-core packages in their 2026
roadmap. Generated types and the OpenAPI snapshot are committed, regenerated in
CI, and required to have a clean diff.

## Detailed design and boundaries

### Contract authority

The authority map is binding:

| Contract concern | Authority | Derived/checking artifact |
|---|---|---|
| REST path and HTTP method | `OpenApiRouter` registration plus handler metadata | OpenAPI `paths` |
| REST JSON field names and representation | Rust wire DTOs with Serde | Utoipa schemas, fixtures, TypeScript types |
| REST parameters, statuses, headers, content types, security | Utoipa path/response metadata tied to the handler | OpenAPI operations and contract tests |
| Runtime bounds and semantic validation | Rust extractor/validator/service code | OpenAPI constraints where representable plus invalid fixtures |
| Lifecycle event envelope/types | JSON Schema files from ADR-005 | Bundled ingestion component inside OpenAPI |
| Domain and database models | Rust domain/storage code | Never exposed automatically |
| Dashboard TypeScript types | Generated from checked OpenAPI | `packages/api-client` generated module |
| Human API semantics and limitations | PRD/system design plus endpoint doc comments | OpenAPI descriptions and reference documentation |

OpenAPI is not used to deserialize requests or authorize callers at runtime.
Serde, explicit validation, privacy enforcement, and authorization remain the
execution path. A schema annotation that says `maximum = 200` is not a security
control unless the runtime validator enforces the same maximum.

### Module and artifact layout

The intended P0 layout is:

```text
crates/landfall-server/src/api/
├── mod.rs                 # API assembly and version prefix
├── router.rs              # OpenApiRouter composition
├── openapi.rs             # Base metadata, modifiers, export/composition
├── error.rs               # Shared ApiError runtime + OpenAPI responses
├── dto/                   # REST-only request/response wire types
├── ingest.rs              # Ingestion handlers and path metadata
├── traces.rs              # Trace handlers and path metadata
├── metrics.rs             # Metrics/comparison/data-quality handlers
├── recommendations.rs     # Disposition handlers
├── reports.rs             # Report handlers/download
├── admin.rs               # Configuration/deletion/status handlers
└── health.rs              # Public liveness/readiness handlers

schemas/openapi/
└── landfall-v1.openapi.json       # Generated, reviewed release snapshot

packages/api-client/src/
├── generated/
│   └── openapi.ts                 # Generated types; never hand-edited
├── client.ts                      # Small Landfall-owned Fetch transport
├── errors.ts                      # Typed transport/API error handling
└── index.ts                       # Stable package exports
```

Domain entities and SQLx records do not derive `ToSchema` merely for
convenience. API DTOs are deliberate boundary types so database columns or
internal diagnostic fields cannot accidentally become public.

### Route composition

Each API module returns an `OpenApiRouter<AppState>`. The root composes modules
under one version prefix before it splits the runtime router and document:

```rust
let (api_router, openapi) = OpenApiRouter::with_openapi(base_openapi())
    .nest("/api/v1/events", ingestion_router())
    .nest("/api/v1/traces", traces_router())
    .nest("/api/v1/metrics", metrics_router())
    .nest("/api/v1/reports", reports_router())
    .split_for_parts();
```

This is illustrative, not the final function naming or exact nesting. The
invariants are:

1. A documented handler is registered once through `utoipa_axum::routes!`.
2. Nesting changes both runtime and OpenAPI paths together.
3. `/api/v1` is the only P0 versioned REST prefix.
4. `/health/live`, `/health/ready`, and the OpenAPI document endpoint are also
   represented explicitly, even though they are not under `/api/v1`.
5. Static SPA fallback, asset files, and an optional docs UI are explicitly
   excluded from the API-path equality test.

`route_service` is not assumed to update OpenAPI; official `utoipa-axum`
documentation states that service nesting can leave paths untouched. Such
routes belong only to the non-API allowlist.

### Stable operation identity

Every public operation declares a unique, explicit lower-camel-case
`operationId`, such as `listTraces`, `getTrace`, `createReport`, or
`deleteTrace`. Generated clients and external integrations may use these names,
so a Rust function/module rename must not change them automatically.

Operation IDs, component schema names, tags, and security-scheme names are
public contract identifiers. DTOs use an explicit stable schema name when a
Rust module path or generic expansion could otherwise change the generated
component. CI rejects missing/duplicate operation IDs and unintended generated
names.

### REST DTO rules

REST DTOs normally derive the relevant combination of:

- `serde::Deserialize` or `serde::Serialize` for runtime wire behavior;
- `utoipa::ToSchema` for JSON bodies;
- `utoipa::IntoParams` for path/query/header parameter groups;
- `utoipa::IntoResponses` or `ToResponse` for reusable responses.

Wire rules are:

- JSON properties use the documented `snake_case` naming already present in the
  system design; Serde and schema renames must agree.
- Request objects reject unknown fields where the endpoint contract is closed.
- Optional, nullable, and defaulted are separate concepts. `Option<T>` and
  `#[serde(default)]` are tested against the emitted schema; `null` is not given
  special meaning unless the runtime can distinguish and implement it.
- A documented default exists only when the runtime applies that same default.
- Integers that can exceed JavaScript safe range are decimal strings with a
  named schema/pattern and explicit unit.
- UUIDs serialize as lowercase canonical strings with `format: uuid`; runtime
  validation additionally enforces UUIDv7 where required by ADR-003.
- Times serialize as RFC 3339 UTC strings with `format: date-time`.
- Enums use explicit stable wire strings. Adding an enum value is reviewed as a
  potentially breaking client change.
- Units appear in names or descriptions; fees, lamports, heights, nanoseconds,
  and milliseconds are never ambiguous.
- Arbitrary `serde_json::Value`, unbounded maps, flattened free-form objects, and
  unrestricted custom metadata are prohibited unless separately reviewed.
- Recursive or discriminated schemas use named component references and
  fixtures; inline shapes are avoided where generator support is ambiguous.

Utoipa supports only part of Serde's attribute surface. A successful derive is
not proof that schema and serialization match. Every custom serializer,
flattened field, tagged enum, optional/null distinction, and manual
`schema_with` receives bidirectional fixtures.

### Requests, responses, and errors

Each operation documents:

- path and method;
- explicit operation ID and tag;
- path, query, and significant header parameters;
- request content type and schema when present;
- every intended success status and body;
- expected authentication, authorization, validation, conflict, rate-limit,
  dependency, and not-found statuses;
- response content types and significant headers;
- privacy/missing-data semantics that affect interpretation.

The shared `ApiError` is one Rust type with both `IntoResponse` and Utoipa
response metadata. Its stable envelope is:

```json
{
  "error": {
    "code": "SCHEMA_VALIDATION_FAILED",
    "message": "The event batch contains invalid fields.",
    "request_id": "0198f0d2-...",
    "details": []
  }
}
```

The schema documents bounded, sanitized `details`; it never permits arbitrary
exceptions or echoes rejected secret values. Reusable component responses cover
the standard `400`, `401`, `403`, `404`, `409`, `413`, `422`, `429`, `500`, and
`503` envelopes, but each operation references only statuses it can meaningfully
produce. Middleware-produced `413`, `429`, `500`, and `503` are included where
applicable even if the handler's Rust return type does not expose them.

Contract tests verify actual response status, content type, headers, and JSON
against the referenced schema. A response enum/annotation is documentation, not
proof that every middleware branch matches it.

Significant header contracts include:

- `Authorization: Bearer ...` on protected operations;
- optional request `Content-Encoding: gzip` on ingestion;
- `Retry-After` on `429` and applicable `503` responses;
- `ETag`, `If-None-Match`, and `304` on projection-backed detail reads;
- `Content-Disposition`, content type, and size behavior on report downloads;
- request ID response/header behavior where implemented.

### Authentication and security metadata

OpenAPI defines two distinct HTTP bearer schemes:

- `IngestToken` for one-environment event submission;
- `AdminToken` for query, configuration, export, and deletion operations.

They are separate schemes because application-defined ingest/admin scopes are
not OAuth scopes and should not be implied by an empty generic bearer scheme.
Every protected operation declares exactly one appropriate requirement. Public
health and OpenAPI-document operations explicitly declare empty security rather
than inheriting ambiguous defaults.

The document describes token prefixes and transport requirements but contains
no usable token, authorization header, cookie, RPC URL, or credential-shaped
example. “Authorize” support in an optional docs UI does not persist credentials
on behalf of Landfall and is disabled by default outside loopback/development.

### Event-schema composition

The ingestion batch request is the deliberate exception to ordinary Utoipa
`ToSchema` ownership. Its export pipeline is:

```text
schemas/events/v1 + manifest + golden fixtures
                  |
                  v
offline deterministic JSON Schema bundler
                  |
                  v
OpenAPI components.schemas.LandfallEventBatchV1
                  ^
                  |
Utoipa ingestion requestBody $ref placeholder
```

The exporter reads only checked-in schemas, validates the manifest/checksum,
bundles local references without network access, and replaces a known placeholder
with the canonical batch schema in the serialized OpenAPI JSON. It preserves
Draft 2020-12 semantics and fails if unsupported keywords would be lost. The
OpenAPI document records event schema version and SHA-256 checksum in bounded
`x-landfall-*` metadata.

The ingestion route may describe HTTP headers, compression, body-size limits,
authentication, and response envelopes in Utoipa. It must not hand-describe the
event envelope, event union, attributes, fingerprint, or their validation rules.
The collector still validates the raw parsed request against the canonical JSON
Schema before semantic checks, as ADR-005 requires.

Golden valid and invalid event batches run through all three consumers:

- the standalone JSON Schema validator;
- the OpenAPI-extracted embedded component;
- the real collector boundary.

Their accept/reject decisions must agree for structural cases.

### Deterministic export pipeline

OpenAPI generation is an explicit offline command, not a network request, build
script with hidden state, or production runtime operation. The intended flow is:

```text
Rust DTOs + Utoipa metadata + OpenApiRouter
                         |
                         v
             raw Utoipa OpenAPI 3.1.0
                         |
canonical event schema -> deterministic composer
                         |
                         v
             validate and policy-lint
                         |
                         v
schemas/openapi/landfall-v1.openapi.json
                         |
                         v
          pinned openapi-typescript
                         |
                         v
packages/api-client/src/generated/openapi.ts
```

The exporter is a small repository binary or tool that needs no database,
secrets, environment-specific base URL, clock, random number, network, or Solana
RPC. It recursively sorts JSON object keys for stable output while retaining
meaningful array ordering and writes pretty UTF-8 JSON with a trailing newline.
It includes no build timestamp, absolute filesystem path, hostname, Git branch,
or developer identity.

Both derived files are committed because they are reviewed release artifacts,
make changes visible in pull requests, support offline builds, and prevent every
consumer from needing Rust tooling. They are never edited by hand.

CI regenerates into a temporary location and byte-compares both files. A dirty
diff fails with the exact regeneration command. Generation also runs from a clean
checkout in the release pipeline to detect undeclared local dependencies.

### Validation and policy linting

The generated document must pass a pinned OpenAPI 3.1 structural validator and
a Landfall policy linter. The exact third-party validator binary is selected and
locked during the protocol/tooling implementation spike; changing validators
does not change contract authority.

The Landfall linter rejects at least:

- missing or duplicate operation IDs;
- undocumented runtime API routes or documented paths absent from the runtime
  API route set, excluding the explicit non-API allowlist;
- protected operations without exactly one expected security requirement;
- public operations without explicit empty security;
- missing standard error schemas for reachable middleware outcomes;
- schemas with unreviewed free-form `additionalProperties`;
- unbounded collection/query limits where the product requires bounds;
- large integer fields represented as unsafe JSON numbers;
- missing units on known numeric quantities;
- non-versioned REST resource paths under `/api`, except the explicit
  `/api/openapi.json` contract endpoint;
- external `$ref` or schema/network dependencies in the release artifact;
- duplicate/unstable schema component names;
- examples or descriptions matching secret/credential fixtures;
- hard-coded deployment URLs, localhost assumptions, or authorization values;
- event-schema version/checksum disagreement with ADR-005 artifacts.

The linter is defense in depth. Runtime behavior is still verified by HTTP
contract tests.

### TypeScript generation and client boundary

The checked OpenAPI snapshot is the only input to TypeScript generation. The
initial generator is `openapi-typescript` 7.13.0, pinned exactly in the workspace
lockfile and configuration. It generates types only; no generated network
runtime executes in the dashboard.

The Landfall-owned client is deliberately small:

- uses the platform `fetch` implementation;
- accepts an explicit base URL and token provider;
- encodes path/query/body values using generated operation types;
- sets content type and request ID behavior centrally;
- supports `AbortSignal` without inventing cancellation semantics;
- parses the shared error envelope and preserves HTTP status/headers;
- never logs authorization or complete sensitive URLs;
- does not add automatic retries to non-idempotent operations;
- exposes endpoint-specific functions whose input/output types are indexed from
  generated `paths` rather than copied interfaces.

The generated file is an internal package detail. Stable exports come through
`packages/api-client/src/index.ts`, so a generator upgrade does not force every
dashboard import to change. Generated code is formatted mechanically only when
that formatting is deterministic and included in the regeneration command.

The Rust CLI does not consume TypeScript. It reuses Rust REST DTOs and a small
Reqwest transport while contract fixtures verify that it sends and accepts the
same wire shapes. The TypeScript package is for the dashboard, examples, and
future JavaScript API consumers.

`openapi-fetch` and a full SDK generator remain alternatives, not hidden
dependencies. Reconsider them only with a measured maintenance benefit and
generated-output/security review.

### API versioning and compatibility

The URI major is `/api/v1`. The OpenAPI `info.version` is a separately maintained
REST contract semantic version, initially `1.0.0`; it is not generated from Git
or a wall clock. Release metadata may state the Landfall build version outside
the deterministic contract.

Within `/api/v1`:

- backward-compatible additions increment the contract minor version;
- documentation-only corrections that do not alter machine behavior increment
  patch when a release artifact is published;
- breaking wire or semantic changes require `/api/v2` and contract major 2,
  unless changed before the first public compatibility promise;
- deprecated operations/fields are marked `deprecated` and documented before
  removal in the next major.

Breaking changes include at least:

- removing or renaming a path, method, operation ID, field, header, or status the
  client relies on;
- adding a required request field or narrowing accepted input;
- making an optional response field required when older servers may omit it;
- changing a field type, unit, nullability, default, pagination, or identifier
  meaning;
- changing authentication/scope requirements;
- removing an enum value or adding one where clients are documented to treat the
  enum as closed;
- changing a status/body combination or idempotency semantics incompatibly.

Adding a new endpoint or an optional response field is normally additive, but
semantic diff output is reviewed rather than accepted blindly. Generated-code
compilation against representative dashboard usage catches changes that a
generic diff tool may misclassify.

Every OpenAPI change includes a contract/changelog decision. CI compares the
new snapshot with the main-branch release baseline using a pinned semantic diff
tool and fails on unapproved breaking changes. A textual snapshot diff remains
required because semantic tools can miss descriptions, examples, extensions,
and generator-sensitive changes.

### Serving the contract and optional documentation UI

The server embeds the checked OpenAPI JSON bytes and serves those exact bytes at
`GET /api/openapi.json`. It does not regenerate or compose the document per
request. The response is read-only, unauthenticated, cacheable, and contains no
deployment/customer data. An ETag/checksum identifies the artifact.

An interactive docs UI may be enabled for local development and portfolio demos.
It must use vendored assets, require no CDN, and be disabled by default for a
non-loopback production binding unless the operator opts in. It never embeds a
token, persists credentials in Landfall storage, enables cross-origin access by
default, or changes API authentication.

Release archives and container images include the JSON snapshot. Operators can
download it without running a generator or accessing the internet.

### What this ADR does not decide

This record does not finalize:

- exact Rust handler/DTO names;
- the exact third-party OpenAPI structural validator and semantic diff binary;
- optional interactive documentation UI product/visual design;
- CORS policy for a future separately hosted dashboard;
- `/api/v2` migration details;
- public npm publishing of `packages/api-client`;
- runtime request validation libraries;
- the event schema fields already owned by ADR-005.

These choices must preserve the ownership, determinism, route-linking, security,
and compatibility rules established here.

## Consequences

### Positive

- Axum route composition and OpenAPI path composition happen together.
- Rust wire DTOs drive both runtime serialization and REST schema generation.
- Event schema authority remains language-neutral and is not duplicated.
- Checked snapshots make API changes visible in code review and release assets.
- The dashboard receives generated strict types without a generated runtime
  networking dependency.
- Explicit operation IDs and schema names remain stable through Rust refactors.
- Contract tests and policy lint catch metadata/runtime gaps that macros alone
  cannot prevent.
- Self-hosted documentation and generation work without internet access.

### Negative

- Handlers require Utoipa annotations in addition to Axum/Serde code.
- A composition step is needed to embed the canonical event schema.
- The project owns a small TypeScript Fetch client and OpenAPI policy linter.
- Compile time and pull-request diffs grow with macros and generated artifacts.
- Code-first design can make large cross-endpoint API redesign less visually
  centralized than a design-first document.
- Utoipa cannot prove that runtime validators, middleware, and error paths match
  the document.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Axum route exists outside OpenApiRouter | Undocumented endpoint | Prohibit direct API routes, route-set equality test, explicit non-API allowlist |
| Annotation disagrees with handler/middleware | Client expects wrong status/schema | Shared response types, HTTP contract fixtures, runtime response validation in tests |
| Serde attribute is not understood by Utoipa | Wire/schema mismatch | DTO rules, custom-serializer fixtures, extracted-schema validation |
| Event schema is copied manually into Utoipa | SDK/collector/docs drift | Mechanical offline injection plus checksum and cross-validator fixtures |
| Generated artifact changes nondeterministically | Noisy diffs and unreproducible releases | No clocks/network/env input, key sorting, clean-checkout regeneration |
| Generator upgrade rewrites public client surface | Dashboard churn or subtle bug | Exact pins, committed output, stable package facade, explicit upgrade review |
| Breaking API change appears additive | Existing clients fail | Semantic and textual diff, compiled consumer fixtures, human compatibility review |
| Security scheme omitted on one operation | Documentation encourages unauthenticated call | Per-operation security linter and real auth tests |
| Secret appears in an example/server URL | Credential disclosure in repo/release | Synthetic fixtures only, secret scan, no environment-derived metadata |
| Interactive docs retain a token | Local credential exposure | Opt-in UI, no Landfall persistence, vendored same-origin assets, production default off |
| OpenAPI is treated as runtime validation | Oversized/invalid input reaches service | Explicit runtime validators, limits, privacy checks, adversarial tests |
| Tool lacks a JSON Schema 2020-12 keyword | Ingestion contract is weakened in docs/client | Fail composition/validation, golden fixtures, upgrade or replace tool before release |

## Security and privacy impact

The OpenAPI document describes endpoint structure but must contain no live
deployment data. Server objects are omitted or use a generic relative origin;
generation never reads configured URLs. Examples are synthetic and pass the
same credential/private-data scanner as reports and fixtures.

Authentication metadata distinguishes ingest and admin bearer credentials.
This is documentation only: middleware still hashes, scopes, expires, revokes,
and authorizes tokens. The generated TypeScript types never contain a credential.
The Fetch client receives a token from the application at call time and redacts
authorization and privacy-sensitive path/query values from its diagnostics.

The OpenAPI JSON endpoint is public because it contains static contract data and
must not rely on obscurity for protection. The optional interactive UI has a
more conservative production default because browser token entry/history and
additional assets expand the attack surface.

Request/response examples, error details, schema descriptions, extensions, and
operation IDs are secret-scanned. Signature lookup remains subject to strict
privacy mode even though the path is documented; documentation explicitly
describes the disabled/hidden response without including a real signature.

No remote documentation service, telemetry, schema registry, CDN, or generator
is contacted during build, export, server startup, or UI use.

## Reliability and failure behavior

OpenAPI generation is a development/release operation. A generation or
validation failure blocks CI/release but does not affect a previously built
server. Production serves embedded checked bytes and has no generator runtime
dependency.

If generated TypeScript is stale, CI regeneration fails before merge. If the
checked OpenAPI artifact is missing, the normal build/release fails rather than
serving a dynamically guessed document. A documentation UI failure does not
affect API routes, ingestion, workers, or dashboard static assets.

Runtime correctness does not depend on a client following the document. The
server continues to authenticate, bound, parse, validate, authorize, and return
structured errors for arbitrary HTTP clients.

New server versions remain compatible with supported `/api/v1` clients under
the versioning policy. The dashboard and server ship together in P0, but that is
not used as permission to make silent breaking changes because CLI, SDK, curl,
reports, and external tooling can live longer than one deployment.

## Performance and capacity impact

Utoipa macros add compile-time work but no per-request schema-generation cost.
The checked JSON is embedded/served as static bytes. TypeScript types erase at
runtime; the Landfall Fetch transport remains small and does not add a general
client framework.

OpenAPI export, validation, semantic diff, and TypeScript generation run in CI
and release workflows, not on the ingestion path. Their target is developer
feedback in seconds, but correctness and deterministic output take priority over
micro-optimizing generation.

The release document is expected to remain a few MiB or less. Event-schema
bundling may be the largest component. CI records artifact size and fails a
review threshold increase so accidental recursive expansion or duplicated
schemas cannot silently bloat the container/dashboard tooling.

Serving the spec and docs UI uses separate static-response limits and cannot
consume worker/database concurrency. Normal API capacity targets remain
unchanged.

## Operational impact

The repository gains one deterministic export command and one contract-check
command. A developer changing a handler/DTO runs them before commit. CI and the
release pipeline run the same commands from pinned lockfiles.

System status exposes the REST contract version, event schema version/checksum,
and OpenAPI artifact checksum without exposing paths containing customer data
or credentials. Release notes state API compatibility changes and attach the
OpenAPI JSON/checksum.

Dependency automation may propose Utoipa, `utoipa-axum`, or
`openapi-typescript` updates, but generation output is never auto-accepted.
Updates require changelog review, regenerated diff, structural/policy validation,
contract fixtures, generated-client compilation, and a compatibility decision.

The embedded snapshot means a developer must regenerate before building after
an intentional API change. CI error messages provide the command rather than
silently rewriting the worktree.

## Verification

ADR-008 is implemented only when all of the following are true:

- All JSON API handlers are registered through `OpenApiRouter`; runtime and
  documented API route sets are equal after removing the explicit non-API
  allowlist.
- Every operation has a unique explicit operation ID, stable component names,
  tag, request definition, expected response set, and security declaration.
- The generated document parses as OpenAPI 3.1.0 with the pinned structural
  validator and passes the Landfall policy linter.
- Regeneration from a clean checkout is byte-identical and produces no Git diff.
- The checked document contains no external references and generation performs
  no network access.
- The embedded event component has the expected schema version/checksum and
  accepts/rejects the same structural golden batches as the canonical JSON
  Schema validator.
- Serde/OpenAPI fixtures cover field renames, unknown fields, optional versus
  nullable, defaults, UUIDs, RFC 3339 times, large decimal-string integers,
  units, enums, pagination, and custom serializers.
- HTTP contract tests cover every documented success status and representative
  `400`, `401`, `403`, `404`, `409`, `413`, `422`, `429`, `500`, and `503`
  branches, validating content type, important headers, and body schema.
- Auth tests prove ingest/admin/public operations match their OpenAPI security
  requirements and real middleware enforcement.
- Middleware failure tests prove body limits, gzip errors, rate limits, request
  IDs, and dependency readiness match documented responses.
- `openapi-typescript` regeneration is clean; generated types and the stable
  package facade compile under strict TypeScript.
- Dashboard representative calls and MSW fixtures compile against the generated
  operation types.
- Rust CLI wire fixtures match the same OpenAPI request/response examples.
- A semantic diff test rejects deliberate breaking fixtures and allows reviewed
  additive fixtures.
- Secret scanning finds no token, credential URL, raw signed transaction,
  private-key fixture, real signature, hostname, or environment value in the
  document, generated TypeScript, or docs UI assets/configuration.
- `GET /api/openapi.json` returns byte-identical embedded content with the
  expected checksum/ETag and no database dependency.
- Optional docs UI works offline and remains disabled by default for the
  documented non-loopback production profile.

## Rollout and migration

There is no production REST specification to migrate. Implementation order is:

1. Pin Utoipa/`utoipa-axum`, create REST DTO and shared response conventions,
   and define base OpenAPI metadata/security schemes.
2. Build API modules with `OpenApiRouter` and explicit operation IDs while
   implementing the first ingestion endpoint.
3. Implement deterministic event-schema bundling and verify ADR-005 fixtures.
4. Export, validate, policy-lint, and commit the first OpenAPI snapshot.
5. Pin `openapi-typescript`, generate/commit types, and add the stable Fetch
   client facade.
6. Add regeneration drift, route-set, HTTP contract, security, semantic diff,
   and secret tests to CI.
7. Embed/serve the snapshot and add the optional offline documentation UI.
8. Add later query/control routes only through the same pipeline.

Producer/consumer rollout for a backward-compatible `/api/v1` addition is:
server accepts/serves the new contract first, generated client/dashboard adopts
it second, and old clients remain valid. A breaking change introduces `/api/v2`
alongside v1 for a documented migration window rather than replacing v1 in
place.

If Utoipa generation must be rolled back, the last accepted OpenAPI snapshot and
matching server/client versions are restored together. The snapshot is not
manually patched to conceal a runtime rollback.

## Reconsideration triggers

- Utoipa/`utoipa-axum` cannot represent a required OpenAPI 3.1 contract without
  fragile post-processing outside the deliberate event-schema exception.
- Route-set or HTTP contract tests repeatedly find annotation/runtime drift that
  route-linked code-first generation cannot control.
- The API becomes independently designed and consumed by several external teams
  before Rust implementation, making design-first governance more valuable.
- OpenAPI 3.2 or a later specification feature becomes required and the selected
  Utoipa line cannot emit it.
- The Landfall-owned Fetch transport duplicates enough endpoints/behavior that a
  reviewed full SDK generator has lower total maintenance and security cost.
- `openapi-typescript` no longer supports the emitted OpenAPI/JSON Schema
  features or strict TypeScript target.
- Event-schema composition cannot preserve Draft 2020-12 semantics across the
  chosen validator and generator toolchain.
- Public compatibility obligations require a formal API review board,
  deprecation service levels, or independently versioned client releases.
- Compile time, artifact size, or generation time materially harms the documented
  development/release workflow.

## References

- [Product requirements: APIs](../product-requirements-document.md#19-apis)
- [Product requirements: security and privacy](../product-requirements-document.md#20-security-and-privacy-requirements)
- [System design: API design](../system-design.md#6-api-design)
- [System design: TypeScript SDK/client structure](../system-design.md#91-typescript-sdk)
- [Technical implementation plan: contracts and code generation](../technical-implementation-plan.md#35-contracts-and-code-generation)
- [Technical implementation plan: collector ingestion API](../technical-implementation-plan.md#13-phase-6--collector-ingestion-api)
- [Utoipa 5.5 documentation](https://docs.rs/utoipa/5.5.0/utoipa/)
- [`utoipa-axum` `OpenApiRouter` documentation](https://docs.rs/utoipa-axum/0.2.0/utoipa_axum/router/struct.OpenApiRouter.html)
- [Utoipa OpenAPI 3.1 implementation](https://docs.rs/utoipa/5.5.0/utoipa/openapi/index.html)
- [OpenAPI Specification 3.1.1](https://spec.openapis.org/oas/v3.1.1.html)
- [`openapi-typescript` repository](https://github.com/openapi-ts/openapi-typescript)
- [`openapi-typescript` 2026 roadmap](https://github.com/openapi-ts/openapi-typescript/discussions/2559)
- [ADR-003: identifier wire forms](003-business-action-trace-attempt-event-and-alias-identifiers.md)
- [ADR-005: JSON Schema event contract and code generation](005-json-schema-event-contract-and-code-generation.md)
- [ADR-006: Solana Kit adapter compatibility](006-solana-kit-first-adapter-and-compatibility-roadmap.md)
