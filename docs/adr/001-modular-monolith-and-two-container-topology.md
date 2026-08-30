# ADR-001: Modular monolith and two-container topology

- **Status:** Accepted
- **Date:** 2026-08-30
- **Decision owners:** Landfall maintainers
- **Related requirements:** G8, R1 self-hosted MVP, FR-INGEST-001, FR-ADMIN-001, NFR-REL-001 through NFR-REL-005, P0 performance and scalability requirements
- **Related ADRs:** ADR-002 and ADR-007 (planned)
- **Supersedes:** None
- **Superseded by:** None

## Context

Landfall receives application-side Solana transaction lifecycle events, observes
their signatures through external RPC endpoints, projects trace state, evaluates
diagnostic rules, serves query APIs and a dashboard, generates reports, and
applies retention policy. These responsibilities have different workloads and
failure modes, so they require explicit internal boundaries.

The P0 product is nevertheless an open-source, self-hosted deployment for one
technical team. It is expected to be implemented and initially operated by a
small team, must start locally through Docker Compose, and has a design point of
approximately one million accepted events per day rather than internet-scale
multi-tenancy. It has no validated requirement for independent component
release cycles, regional placement, or contractual high availability.

Leaving deployment boundaries implicit would let source-code modules turn into
premature network services, or let an unstructured monolith entangle domain,
storage, RPC, and HTTP concerns. Either outcome would make failure semantics,
local installation, testing, and future extraction harder to reason about.

## Decision drivers

- A new user should be able to start the self-hosted product with Docker Compose
  and obtain a first trace in under 30 minutes.
- P0 is a single-team product without hosted tenant isolation, RBAC, billing, or
  a remote Landfall control plane.
- The ingestion API must atomically persist accepted events and durable
  background work before returning `202 Accepted`.
- Telemetry must remain outside the customer's Solana submission critical path.
- Acknowledged events and queued jobs must survive application restarts.
- The planned load—one million events/day and a 500-event/second collector
  burst—is within the intended capacity of one Rust server and PostgreSQL.
- One small engineering team must be able to build, test, deploy, observe, and
  explain the entire system.
- Component boundaries must remain strong enough to permit later process
  separation when measurements justify it.
- The self-hosted product must minimize exposed network services and must not
  send customer telemetry to a Landfall-operated service by default.

## Options considered

### Option A — Modular monolith plus a separate PostgreSQL container

Build one Rust server binary with explicit API, projection, observation,
diagnosis, query, report, retention, and static-asset modules. Run all roles in
one server process for P0. Run PostgreSQL as the second required container and
use it for durable storage and background-job coordination.

This option keeps synchronous module calls in-process, permits an atomic
event-plus-job database transaction, and gives P0 the smallest production-like
operational surface. Its principal cost is that one server crash or resource
contention can temporarily affect every application role.

### Option B — Independent microservices and an external message broker

Deploy ingestion, projection, observation, query, reporting, and retention as
separate services, normally with Kafka, NATS, or another broker between them.
This permits independent scaling and fault isolation, but creates distributed
transactions, message-version compatibility, service discovery, more secrets,
more network failure modes, and substantially more deployment work.

P0 has neither measured load nor team boundaries that compensate for that cost.
The broker would also duplicate durable coordination capabilities already
needed in PostgreSQL.

### Option C — One container containing both application and PostgreSQL

Package the Rust process and PostgreSQL into one container. This appears simpler
because there is only one container, but it combines two process lifecycles,
complicates health checks and graceful shutdown, encourages database files to be
tied to an application image, and makes backup, upgrade, resource allocation,
and future database hosting unnecessarily fragile.

The application is replaceable compute; PostgreSQL owns durable state. They
should not share one container lifecycle.

### Option D — Multiple role-specific application containers from the start

Build one Rust binary but run API, projector, observer, reporter, and retention
roles as separate containers against the same PostgreSQL database. This is less
complex than independent microservices and is the preferred first scaling step.
For P0, however, it still multiplies configuration, health checks, logs, resource
limits, and upgrade coordination before resource interference has been measured.

### Option E — Hosted serverless or managed control plane

Send telemetry to a Landfall-operated cloud service and use managed functions,
queues, and storage. This could simplify installation for some users, but it
conflicts with the validated P0 self-hosted/privacy position and introduces
tenant isolation, billing, regional data handling, and external availability
requirements that belong to a later product stage.

## Decision

Landfall P0 shall use a **modular monolith deployed with two required runtime
containers**:

1. `landfall-server`: one Rust binary and, by default, one process containing
   the ingestion/query API, projector, Solana observer, diagnostic engine,
   reporter, retention worker, and compiled dashboard assets;
2. `postgres`: PostgreSQL containing durable raw events, relational projections,
   configuration, idempotency records, background jobs, and P0 report artifacts.

The TypeScript SDK runs inside the customer's Node.js application and is not a
Landfall deployment container. The CLI is an on-demand client. Solana RPC
providers are external dependencies. A TLS reverse proxy and a backup job may be
added by an operator, but they are not required Landfall application components.

The supported P0 deployment mechanism is Docker Compose. Both required
containers may run on one host, but the decision does not require them to do so.
Only PostgreSQL requires a persistent product data volume.

`landfall-server` shall support explicit role selection, conceptually
`--roles api,projector,observer,reporter,retention`. P0 enables all roles in one
process. If measurements later require isolation, the same binary may be run in
multiple role-specific processes before introducing new service protocols or an
external broker.

## Detailed design and boundaries

The default production flow is:

```text
Customer Node.js application + Landfall SDK
                    |
                    | asynchronous redacted event batches
                    v
        +---------------------------+
        | landfall-server           |
        |                           |
        | API -> application ports  |
        |          |                |
        |          +-> projector    |
        |          +-> observer ----+----> external Solana RPC
        |          +-> reporter     |
        |          +-> retention    |
        |                           |
        | query API + static UI     |
        +-------------+-------------+
                      |
                      | SQL transactions and job leases
                      v
              +---------------+
              | PostgreSQL    |
              | durable state |
              +---------------+
```

The single process is a deployment boundary, not a license for arbitrary source
dependencies. The planned Rust workspace shall enforce these directions:

- `landfall-protocol` owns wire types and has no application, database, or web
  dependencies;
- `landfall-core` owns deterministic domain behavior and may depend on protocol,
  but not Axum, SQLx, React, or provider clients;
- storage, observer, and report adapters depend inward on protocol/core
  interfaces;
- `landfall-server` is the composition root that wires HTTP and worker roles to
  those interfaces;
- the dashboard communicates only through documented query APIs and never
  accesses PostgreSQL directly.

Durable communication between asynchronous roles uses PostgreSQL jobs. Direct
in-process calls are allowed for synchronous application services when they do
not bypass domain invariants or persistence guarantees. No private, in-memory
queue may be the sole record of acknowledged work.

Dashboard assets are built from a separate frontend source package and served by
the Rust server in production. A frontend development server and API proxy may
be used locally; that development process does not change the production
topology.

This ADR decides the P0 process and deployment boundaries. It does not decide
the event persistence model, job-table schema, event contract, OpenAPI source of
truth, exact crate APIs, or container base images. Those choices belong to later
ADRs and implementation tasks.

## Consequences

### Positive

- A user installs and operates only one application service plus PostgreSQL.
- In-process calls avoid unnecessary serialization, network latency, service
  discovery, and partial network failures between application components.
- The collector can persist events and projection jobs in one PostgreSQL
  transaction without a database/broker dual-write problem.
- One build artifact makes version compatibility, local debugging, releases,
  and portfolio demonstrations straightforward.
- A single HTTP origin can serve the API and dashboard, reducing production CORS
  and frontend deployment configuration.
- Explicit module and role boundaries preserve an incremental path to separate
  processes without paying distributed-system costs immediately.

### Negative

- A server process crash pauses ingestion, projection, observation, queries, and
  report processing until restart.
- A CPU-heavy report or poorly bounded observer could interfere with ingestion
  unless concurrency and resource use are controlled.
- Application roles cannot be scaled independently in the default topology.
- Deploying a new server version restarts all application roles together.
- Shared process memory increases the blast radius of an application-level
  vulnerability or runaway allocation.
- The application binary and its integration test surface are larger than those
  of a narrowly scoped service.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Source modules become tightly coupled | Later extraction and independent testing become expensive | Enforce crate dependency direction; keep domain core free of web/database/provider dependencies; review boundary violations in CI |
| Report or observer work starves ingestion | SDK buffers fill, telemetry is dropped, or ingestion latency misses its target | Use bounded concurrency, role-specific limits, queue priorities, latency/lag metrics, and performance tests with concurrent workloads |
| Server crash stops every role | Temporary ingestion and observation gap | SDK fails open and retries within a bounded buffer; PostgreSQL jobs use leases; orchestrator restarts the process; surface coverage gaps |
| PostgreSQL outage stops all durable work | API cannot acknowledge events and queries/workers fail | Return `503` without false durability claims; keep readiness false; reconnect with bounded backoff; document backup and restore |
| “Monolith” is interpreted as one unstructured module | Domain logic becomes coupled to Axum and SQLx | Maintain separate crates, explicit application ports, dependency rules, and pure-core tests |
| P0 topology survives beyond its measured limits | Queue lag, query latency, or availability becomes unacceptable | Publish capacity metrics and apply the reconsideration triggers below |

## Security and privacy impact

The self-hosted two-container topology keeps telemetry within the customer's
chosen deployment boundary and requires no Landfall-operated control plane.
`landfall-server` is the only required application network entry point.
PostgreSQL must remain on a private Compose network and must not be publicly
exposed by the provided configuration.

Serving the dashboard and APIs from one origin reduces cross-origin policy and
token-handling complexity. Ingestion and administrative capabilities still use
separate scoped credentials; sharing a process does not authorize one interface
to bypass another interface's policy.

The Rust server has access to several sensitive classes of telemetry and to the
database, so a server compromise can affect all application roles. Mitigations
include a non-root container, a least-privilege PostgreSQL role, secret injection
through deployment mechanisms, allowlisted/redacted telemetry, bounded inputs,
scrubbed logs, and a read-only container filesystem where practical.

Private keys and signing operations remain in the customer application. The SDK
may observe only allowed metadata and must never route transaction submission
through `landfall-server`.

## Reliability and failure behavior

P0 explicitly does not promise high availability. A server restart may
temporarily stop all application roles. The SDK does not synchronously depend on
Landfall, so the customer's Solana transaction path continues independently.

`202 Accepted` is returned only after accepted raw events and corresponding
durable work commit to PostgreSQL. After a server crash, acknowledged events
remain available and expired job leases allow workers to resume. In-memory
caches and schedules are reconstructible; they are never the source of truth for
acknowledged work.

If PostgreSQL is unavailable, the server remains live but not ready, ingestion
returns `503` rather than claiming durability, and workers stop or back off.
Unacknowledged SDK events follow the documented bounded retry/drop policy.

Graceful shutdown must stop accepting new work, finish or release bounded
in-flight operations, and leave leased jobs recoverable. A single-process deploy
causes a short application-wide interruption; zero-downtime rolling deployment
is not a P0 guarantee.

## Performance and capacity impact

The topology is selected for the P0 design point:

- approximately 50,000 transaction traces/day;
- approximately one million accepted events/day;
- 500 accepted events/second planned collector burst;
- about 10 ingestion requests/second at that burst with 50-event batches;
- fewer than 20 persisted events for an ordinary trace;
- a reference application allocation of 2–4 vCPU and 4–8 GiB RAM;
- a reference PostgreSQL allocation of 4 vCPU, 8–16 GiB RAM, and 100 GiB SSD.

In-process composition removes network hops between API and domain services, but
PostgreSQL remains on all durable paths. Benchmarks must therefore include pool
contention, event-plus-job transaction latency, projection lag, observer work,
report generation, and query traffic at the same time—not isolated endpoint
throughput alone.

The default process must meet the documented 250 ms batch-ingestion p95,
five-second projection-lag p95, and 500-event/second collector target on a
published reference environment before P0 release.

## Operational impact

The supported Compose stack has two required services and one required durable
volume owned by PostgreSQL. Operators configure database credentials, scoped API
tokens, RPC endpoints, privacy policy, retention, and role concurrency through
documented configuration and secret inputs.

The server exposes separate liveness, readiness, and authenticated system-status
views. Logs and metrics identify the internal role so one process remains
diagnosable. PostgreSQL migrations complete before readiness becomes true.

Database backup, restore, retention, and upgrade procedures are independent of
application image replacement. Server containers are replaceable and retain no
authoritative local state. A reverse proxy is required for public TLS exposure
unless the product is placed behind an existing trusted ingress or private
network.

The first operational scaling step is to run the same binary with distinct role
sets against the same PostgreSQL queue. That step requires its own deployment
profile and evidence, but does not initially require new domain contracts.

## Verification

- A fresh documented environment starts exactly the two required services with
  Docker Compose and reaches readiness.
- One server binary can start all default roles and serve compiled dashboard
  assets; role flags can start a selected subset without linking a second binary.
- Automated dependency checks or architecture tests prevent protocol/core crates
  from importing Axum, SQLx, React, or Solana provider adapters.
- An SDK-to-dashboard integration test records a transaction trace, projects it,
  observes it through a mock or controlled Solana endpoint, queries it, and
  renders it using the two-container stack.
- Restart tests prove that an acknowledged event survives server termination and
  that an expired worker lease is reclaimed.
- PostgreSQL-unavailable tests prove that ingestion returns `503`, readiness is
  false, and no `202 Accepted` is emitted.
- Concurrent ingestion, observation, query, and report benchmarks meet the P0
  latency/throughput targets on a published hardware profile.
- Deployment tests prove PostgreSQL has no published public port by default and
  that the server process runs without private keys or raw signed transactions.

## Rollout and migration

This is the initial implementation topology, so no existing Landfall deployment
or data requires migration. Phase 1 creates the modular workspace and minimal
Compose stack following this decision.

If roles are later separated, operators may first deploy the same version of the
same binary with disjoint role flags, wait for old leases to expire or complete,
and then disable those roles in the original process. PostgreSQL remains the
coordination boundary. Moving to independently released services or a broker
requires a superseding ADR and an explicit protocol/data migration plan.

## Reconsideration triggers

Revisit the process or deployment topology when measured evidence shows any of
the following:

- more than 10 million accepted events/day;
- retained PostgreSQL data above 500 GiB;
- projection lag above 30 seconds during normal load;
- common 24-hour queries above two seconds after appropriate indexing;
- reports or observer work materially degrade ingestion latency despite bounded
  concurrency;
- observer workloads consistently exceed RPC quotas and require independent
  regional or endpoint placement;
- a contractual high-availability or zero-downtime deployment requirement;
- hosted multi-tenancy requiring isolation, independent scaling, or regional
  data placement;
- separate teams require independent ownership and release cycles;
- security analysis requires a smaller process-level blast radius.

Crossing a trigger requires measurement and design review; it does not
automatically imply microservices. The smallest adequate response may be
role-specific processes using the same binary and PostgreSQL queue.

## References

- [Product requirements: product scope and self-hosted MVP](../product-requirements-document.md#112-r1--open-source-self-hosted-mvp-p0)
- [Product requirements: non-functional requirements](../product-requirements-document.md#21-non-functional-requirements)
- [System design: deployment model and architectural style](../system-design.md#22-p0-deployment-model)
- [System design: high-level design](../system-design.md#7-high-level-design)
- [System design: capacity estimation](../system-design.md#5-capacity-estimation)
- [System design: deployment design](../system-design.md#10-deployment-design)
- [Technical implementation plan: repository structure and Phase 0](../technical-implementation-plan.md#4-planned-repository-structure)
