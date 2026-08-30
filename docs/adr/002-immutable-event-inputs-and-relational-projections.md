# ADR-002: Immutable event inputs and relational projections

- **Status:** Accepted
- **Date:** 2026-08-30
- **Decision owners:** Landfall maintainers
- **Related requirements:** FR-INGEST-001 through FR-INGEST-003, lifecycle state model, event schema, NFR-REL-002, P0 performance and scalability requirements
- **Related ADRs:** [ADR-001](001-modular-monolith-and-two-container-topology.md); ADR-003, ADR-005, and ADR-007 (planned)
- **Supersedes:** None
- **Superseded by:** None

## Context

Landfall must explain a Solana transaction lifecycle using incomplete facts that
arrive from different sources and at different times. The customer application
can report construction, simulation, signing, submission, retry, and business
outcome. An observer can later report block height, signature status, landing,
execution, and commitment. Either source can be delayed, duplicated, missing, or
temporally inconsistent with another source.

The product must preserve the evidence behind each conclusion. RPC acceptance
must not silently become “landed,” a late observation must be able to revise a
previously incomplete conclusion, and a rule fix must be applicable to retained
evidence without rewriting history. At the same time, dashboard and metric APIs
need typed, indexed, low-latency data rather than rebuilding every trace from
JSON during each request.

If Landfall stores only the current state, it loses the facts needed to audit or
reproduce that state. If it stores only raw events, every query must repeat
ordering, identity resolution, lifecycle reduction, and diagnostic evaluation.
The persistence model therefore needs both durable evidence and efficient read
models, with an explicit consistency contract between them.

## Decision drivers

- Every authoritative lifecycle state, diagnosis, and recommendation must link
  back to retained evidence.
- Duplicate and out-of-order delivery must converge on the same result.
- Late evidence must update the current view without erasing prior facts.
- Rule and reducer changes must be testable through deterministic replay.
- Ingestion must remain bounded and must not synchronously perform full trace
  analysis.
- Query APIs must meet interactive latency targets using typed indexed fields.
- Metrics must use one versioned definition rather than reimplementing event
  interpretation in the dashboard.
- PostgreSQL must remain sufficient for the P0 operational footprint and design
  point of approximately one million events/day.
- Raw signed transactions, credentials, and unbounded logs must not be retained
  merely to make replay possible.
- Retention and explicit deletion must remain enforceable despite append-only
  event semantics.

## Options considered

### Option A — Immutable raw events plus derived relational projections

Persist each accepted telemetry fact as an append-only raw event. Asynchronous
projectors load the retained events for a canonical trace, run a deterministic
versioned reducer, and transactionally write typed relational read models.

This preserves evidence and enables replay while keeping common queries fast.
Its costs are duplicate storage, eventual consistency, projection workers, rule
versioning, and operational tooling for replay and lag monitoring.

### Option B — Store and synchronously update only current relational state

Translate each incoming request directly into updates such as
`transaction_traces.state = 'confirmed'`. This is easy to query and initially
uses less storage, but it discards how the state was reached. Out-of-order events
become conditional-update logic, rule bugs cannot be repaired from evidence, and
an overwrite can silently erase contradictory observations.

An additional audit table would gradually recreate an event log while retaining
the complexity of mutable current-state writes.

### Option C — Store only raw events and derive every response on demand

Append events but do not persist read models. Each API request loads and reduces
the relevant event set. This minimizes projection infrastructure and ensures the
latest rules are always used, but makes latency and metric consistency depend on
repeated computation. Search, cohort comparison, pagination, and aggregation
over many traces become expensive or require ad hoc caches that are projections
under another name.

### Option D — Mutate or correct raw events in place

Allow later evidence or administrative action to update an earlier raw row. This
can make the latest row look clean, but destroys the original audit trail,
complicates deduplication, makes historical diagnoses irreproducible, and hides
whether a value was observed initially or corrected later.

Corrections should be represented as additional versioned facts or controlled
deletion, not invisible mutation.

### Option E — External event store/broker and analytical database

Place events in Kafka or a dedicated event store and serve projections from
ClickHouse, Elasticsearch, or another database. This may be appropriate at much
higher throughput or retention, but adds service operation, schema compatibility,
replay coordination, backup boundaries, and data duplication without a measured
P0 need. PostgreSQL can provide the required transactions, partitions, JSONB,
typed tables, and workload capacity.

## Decision

Landfall shall use **immutable, append-only telemetry events as the retained
source of truth for transaction-lifecycle evidence** and **versioned typed
relational projections as the query model**.

For every accepted event, the collector shall persist the validated/redacted
event envelope without later application-level `UPDATE`. A correction,
contradictory observation, or later state is represented by another event. An
event can be removed only through an explicit retention, user-authorized
deletion, or privacy-incident procedure; immutability does not mean indefinite
retention or immunity from lawful deletion.

Projection workers shall asynchronously rebuild the current state of an affected
canonical trace from all relevant retained events. The reducer and diagnostic
rules shall be deterministic and versioned. Projected rows shall record their
projection/rule version and processing watermark.

The ingestion API shall not synchronously calculate the authoritative
projection. `202 Accepted` means non-duplicate events and corresponding durable
work committed; queryable lifecycle state may still be processing. APIs and the
dashboard shall expose projection freshness rather than render missing work as a
terminal failure.

Both raw events and relational projections shall live in PostgreSQL for P0. This
decision applies to transaction telemetry and derived product outputs, not to
every piece of Landfall state: project/environment configuration and token
records are authoritative relational control state, while jobs are mutable
operational coordination records.

## Detailed design and boundaries

### Data ownership

| Data class | Examples | Authority | Mutation model |
|---|---|---|---|
| Telemetry evidence | SDK lifecycle events, observer status evidence | Raw event rows while retained | Insert-only; controlled deletion only |
| Current trace read model | lifecycle stage, landing, execution, commitment | Derived from retained evidence and versioned rules | Transactionally replace/upsert |
| Historical derived conclusions | diagnoses, evidence links, rule version | Derived but retained for audit according to policy | Append/supersede rather than silently rewrite |
| Control state | projects, environments, routes, token hashes | Relational control tables | Validated create/update/revoke operations |
| Work coordination | projection/observation/report jobs and leases | Job table | Mutable state machine with idempotent effects |
| Report artifact | sanitized HTML/JSON snapshot | Derived from a named projection/evidence version | Create, expire, delete |

The phrase “event-sourced inputs” is deliberately narrower than claiming that
Landfall is a fully event-sourced application. Commands, configuration, access
tokens, job leases, and every administrative edit do not need to be reconstructed
from the telemetry event log.

### Write and projection flow

```text
SDK or observer
      |
      | validated, redacted event
      v
Event persistence transaction
      +-- claim event_id in dedup registry
      +-- insert immutable raw event
      +-- enqueue/merge project_trace work
      |
      v
202 Accepted: evidence durable, projection pending
      |
      v
Projector
      +-- resolve canonical trace
      +-- load all relevant retained events
      +-- order deterministically
      +-- run versioned reducer and rules
      +-- write typed projection atomically
      +-- advance watermark/version
      |
      v
Query API and dashboard
```

The exact job-table mechanics belong to ADR-007. The binding guarantee here is
that accepted evidence cannot be acknowledged without durable projection work,
and projection work can be safely retried.

### Raw event invariants

Each raw event shall include a globally deduplicated event ID, schema version,
event type, project/environment ownership, source timestamps, collector receipt
time, source identity/version, privacy/redaction version, trace/business-action
identity where available, and bounded allowlisted attributes.

The storage interface shall provide no ordinary “update raw event” operation.
Database-level privileges, constraints, or triggers shall prevent accidental
updates. Retention and authorized deletion use explicit narrowly scoped paths
and must remove dependent searchable/derived data according to policy.

Raw events are time-partitioned by collector receipt date. A separate dedup
registry preserves global `event_id` idempotency across partition boundaries and
remains for the raw retention period plus a safety window.

Raw event JSONB is evidence storage, not a general query API. Frequently filtered
or aggregated values shall be validated into typed projected columns. Arbitrary
high-cardinality GIN indexing over the full attribute object is not the default.

### Projection invariants

A projection is a cache of an authoritative versioned computation, not an
independent source of lifecycle truth. It must be possible to discard and rebuild
it from all required retained events, the reducer/rule version, and referenced
control configuration valid for that computation.

For P0, a `project_trace` run rebuilds the complete canonical trace rather than
applying a single event incrementally. An ordinary trace is expected to contain
fewer than 20 persisted events, making full reduction inexpensive and much safer
for out-of-order and late delivery.

Event ordering shall be deterministic and shall use defined semantic time,
collector receipt time, event-type semantics where required, and event ID as a
stable tie-breaker. Clock uncertainty remains data-quality evidence; sorting
must not manufacture a precise cross-system order that the timestamps cannot
support.

All related projection changes for one trace—trace summary, attempts,
observations, execution metadata, current diagnoses, recommendations, evidence
links, and watermark—shall commit atomically. A reader sees either the previous
complete projection or the next complete projection, never a partial mixture.

Projection metadata shall include at least:

- monotonically increasing projection version for the trace;
- reducer/rule-set version;
- time or event watermark through which input was processed;
- projection completion/update timestamp;
- data-quality/completeness state where applicable.

Query endpoints shall return an `as_of` or equivalent watermark when freshness
matters. The dashboard shall display `processing` when accepted evidence is newer
than the available projection.

### Replay and history

Replay can target one trace, a bounded time range, or a new reducer/rule version.
The same canonical event set and same rule version must produce the same
authoritative result. Replay shall be idempotent and shall not duplicate attempts,
observations, metric contributions, diagnoses, or recommendations.

A later event may change the current projected state, including after a prior
terminal or unknown conclusion. Prior raw evidence is never erased by that
change. Historical diagnoses that are retained must be marked superseded and
must keep their rule/evidence references rather than being presented as current.

Replay guarantees apply only while the required raw events and referenced
configuration are retained. P0 may retain raw events for 14 days and projections
for 30 days. An older projection can therefore remain queryable after its full
raw replay window has expired; its recorded version and evidence availability
must not imply that a complete replay is still possible.

This ADR does not define the exact event schema, identifier canonicalization,
database DDL, job leasing algorithm, or reducer state machine. Those details
belong to later ADRs and implementation phases.

## Consequences

### Positive

- Every current conclusion can reference the evidence and rule version that
  produced it while that evidence is retained.
- Duplicate, late, and out-of-order events can converge through full replay.
- Reducer and rule bugs can be fixed and evaluated against historical fixtures
  without editing the original facts.
- Relational projections support fast trace lookup, filtering, pagination,
  metrics, cohort comparison, and reporting with explicit typed semantics.
- UI and report code consume one authoritative query model instead of
  independently interpreting event JSON.
- PostgreSQL transactions keep each trace projection internally consistent.

### Negative

- Raw and derived representations consume more storage than either alone.
- Users may briefly see a processing state because ingestion and projection are
  eventually consistent.
- Projector code, replay tooling, version metadata, lag metrics, and dead-job
  handling add implementation and operational complexity.
- A rule change does not automatically make old exported reports current; their
  source versions must remain visible.
- Full reproducibility ends when required raw evidence is deleted by retention.
- The team must evolve event schema and projection migrations independently but
  compatibly.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Sensitive value is accepted into immutable storage | Privacy or credential exposure persists until explicit deletion | Allowlisted schemas, SDK and server redaction, prohibited-value tests, raw signed transactions off by default, bounded retention, incident deletion runbook |
| Projector bug makes projections drift from raw evidence | Incorrect lifecycle, metrics, or recommendations | Deterministic golden fixtures, full-replay comparisons, versioned rules, evidence links, reproject command |
| Partial projection write | Trace summary disagrees with attempts/diagnosis | One PostgreSQL transaction per trace projection; rollback on any child-write failure |
| Out-of-order or contradictory events are forced into a false sequence | Incorrect diagnosis confidence | Deterministic ordering plus clock-quality metadata; preserve contradiction and return unknown/probable rather than invent certainty |
| Replay storm overloads ingestion or queries | Increased latency and growing lag | Bounded replay ranges, separate concurrency limits/priorities, lag metrics, resumable jobs |
| Retention deletes raw data but leaves sensitive derived/search data | Incomplete deletion and privacy breach | Coordinated retention/deletion job, dependency inventory, post-delete verification tests, partition-aware cleanup |
| Raw storage/index growth exceeds estimates | Vacuum, disk, and query degradation | Daily partitions, bounded payloads, selective indexes, storage metrics, enforced retention, capacity triggers |
| Rule upgrade silently rewrites historical meaning | Audit reports cannot be reproduced | Store reducer/rule version, supersede retained diagnoses, snapshot report metadata, explicit versioned replay |

## Security and privacy impact

Immutable storage increases the consequence of accepting a prohibited value.
Privacy enforcement must therefore occur before the raw insert in both the SDK
and collector; server validation is authoritative. Private keys, seed phrases,
credentials, authorization headers, cookie values, credential-bearing URLs, and
raw signed transaction bytes are prohibited from default event storage.

Event attributes are allowlisted and size-bounded. Raw partitions, dedup rows,
derived identifiers, diagnoses, search indexes, and report references must all
participate in retention and deletion. Database backup limitations must be
disclosed because deleting live rows does not immediately rewrite historical
backups.

Read/query credentials should not automatically grant raw-event access. The
dashboard consumes redacted projections through APIs. Administrative raw-event
inspection, if implemented, requires a narrower privileged path and audit
controls.

Immutability is subordinate to explicit privacy deletion. Deletion is visible as
an administrative/audit action where policy permits, but Landfall must not retain
the sensitive deleted payload merely to prove what was deleted.

## Reliability and failure behavior

The collector returns `202 Accepted` only after each new event and corresponding
durable work commit. If PostgreSQL is unavailable or the transaction rolls back,
the API returns an error and makes no durability claim.

If a projector crashes before its projection transaction commits, readers keep
the prior complete projection and the job can be retried after its lease expires.
If it crashes after commit but before job completion, idempotent replay produces
the same projection without duplicate child records or metric contributions.

Duplicate `event_id` delivery is harmless. A partial-duplicate batch inserts only
new events, reports duplicate counts, and schedules enough work to incorporate
the new evidence. Late events create another projection cycle and may supersede
the current conclusion.

Projection lag is expected but observable. APIs distinguish “evidence accepted,
projection pending” from “no evidence” and from terminal transaction failure.
A poison event that passed ingestion validation cannot block unrelated traces;
bounded retries eventually place its job in a visible dead state for review.

## Performance and capacity impact

At the P0 design point, Landfall plans for approximately one million raw events
per day. At an estimated 2.2 KiB effective PostgreSQL footprint per raw event,
14-day raw retention is about 31 GiB. Derived data is estimated around 8 KiB per
trace, or about 12 GiB for 30 days at 50,000 traces/day, before general database
operational headroom.

Daily raw-event partitions make bounded retention and time-window scans
predictable. Trace/event indexes support projector reads; typed projected indexes
support query APIs. Full JSONB GIN indexing is avoided unless measured access
patterns justify it.

Rebuilding fewer than 20 events per ordinary trace is expected to be cheaper and
safer than maintaining a complex incremental reducer. Benchmarks must verify the
500-event/second collector burst, projection-lag p95 under five seconds, trace
detail p95 under 500 ms, and 24-hour overview p95 under two seconds on the
published reference profile.

## Operational impact

Operations must expose accepted-event rate, duplicate/rejected counts,
projection queue depth, projection lag, per-rule version counts, failed/dead
jobs, replay progress, partition size, retention status, and database capacity.

The CLI or an equivalent administrative interface shall support bounded
reprojection for a trace, time window, and rule version, with dry-run or impact
information where appropriate. Reprojection must be schedulable so it cannot
silently starve ingestion and live observation.

Database migrations must consider two independently evolving shapes: raw event
versions and projected relational schemas. A new projection may be backfilled
from retained events; an event schema change requires explicit compatibility or
upcasting rules defined by ADR-005.

Backup and restore capture raw events, projection state, rule/version metadata,
and jobs consistently. After restore, projections can be replayed where evidence
is retained. Retention normally drops old raw partitions in bounded operations
and then cleans associated dedup rows after the safety window.

## Verification

- Replaying the same canonical events with the same reducer/rule version produces
  byte-equivalent or semantically identical authoritative projections.
- Every meaningful permutation of duplicate and out-of-order golden events
  converges on the expected projection and metric contribution.
- A late terminal or contradictory event updates the current projection while
  preserving retained prior evidence and superseded diagnosis metadata.
- Forced projector termination before and after projection commit proves there
  is no partial projection and retry creates no duplicate derived records.
- A rule-set upgrade preserves the old diagnosis version and creates the expected
  new version from the same retained evidence.
- Query contracts expose watermark/freshness and show processing when accepted
  evidence is ahead of the projection.
- Database enforcement rejects ordinary raw-event `UPDATE`, while tested
  retention and authorized deletion remove all required raw and derived targets.
- Secret/privacy fixtures are absent from raw events, projections, logs, search,
  reports, and exported artifacts.
- Synthetic concurrent ingestion, projection, query, observation, and replay
  workloads meet the documented capacity targets.

## Rollout and migration

This is the initial telemetry persistence model, so there is no existing
Landfall data migration. Implementation proceeds in this order: versioned event
fixtures and reducer semantics, PostgreSQL raw/dedup tables, relational projection
tables, transactional ingestion, projector jobs, and replay tooling.

Projection schema changes shall use reviewed database migrations and, when
required, a bounded backfill/replay. Event contract changes do not rewrite old
raw rows; readers support the retained schema versions or apply explicit
versioned upcasters.

Moving raw events to an external store or analytical database requires a
superseding ADR, dual-read/write migration design, evidence completeness checks,
and proof that historical rule/evidence references remain valid.

## Reconsideration triggers

Revisit full-trace replay, PostgreSQL storage, or projection strategy when
measurements show any of the following:

- ordinary traces regularly contain hundreds or thousands of retained events;
- more than 10 million events/day or retained database size above 500 GiB;
- projection lag above 30 seconds under normal load after query/index tuning;
- projector reads or replay materially interfere with ingestion durability;
- common 24-hour queries remain above two seconds after appropriate projection
  and indexing work;
- retention or audit requirements demand economical multi-year raw evidence;
- hosted multi-tenancy requires tenant-isolated event storage or regional data
  placement;
- a new product requirement needs global event-stream consumers independent of
  PostgreSQL.

Crossing a trigger does not automatically select Kafka or ClickHouse. Evidence
must identify whether the bottleneck is event storage, projection computation,
analytical querying, operational isolation, or retention economics before the
architecture changes.

## References

- [Product requirements: event ordering and idempotency](../product-requirements-document.md#fr-ingest-003--event-ordering-and-idempotency-p0)
- [Product requirements: data model and event schema](../product-requirements-document.md#16-data-model)
- [Product requirements: storage and processing model](../product-requirements-document.md#187-storage-strategy)
- [System design: processing consistency](../system-design.md#78-processing-consistency)
- [System design: database design](../system-design.md#8-database-design)
- [System design: capacity estimation](../system-design.md#5-capacity-estimation)
- [Technical implementation plan: domain core](../technical-implementation-plan.md#10-phase-3--deterministic-domain-core)
- [Technical implementation plan: projection pipeline](../technical-implementation-plan.md#14-phase-7--projection-and-diagnosis-pipeline)
