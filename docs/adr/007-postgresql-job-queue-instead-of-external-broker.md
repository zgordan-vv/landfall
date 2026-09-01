# ADR-007: PostgreSQL job queue instead of an external broker

- **Status:** Accepted
- **Date:** 2026-09-01
- **Decision owners:** Landfall maintainers
- **Related requirements:** FR-INGEST-001, FR-INGEST-003, FR-OBS-001 through FR-OBS-003, FR-REPORT-002, FR-ADMIN-001, system-design NFR-REL-005 through NFR-REL-007
- **Related ADRs:** [ADR-001](001-modular-monolith-and-two-container-topology.md), [ADR-002](002-immutable-event-inputs-and-relational-projections.md), [ADR-005](005-json-schema-event-contract-and-code-generation.md)
- **Supersedes:** None
- **Superseded by:** None

## Context

Landfall acknowledges an ingestion batch only after immutable events are durable,
but projections are intentionally asynchronous. The same application also needs
durable observation, enrichment, report, deletion, and retention work. A server
restart must not forget that work, and multiple workers must be able to share it
without processing the same row concurrently under normal conditions.

ADR-001 selected a modular monolith with two required containers: the Landfall
server and PostgreSQL. ADR-002 separated immutable evidence from mutable work
coordination and established that accepted events and projection work must be
committed without a database/broker dual-write gap. The P0 capacity model is
approximately one million events and 50,000 traces per day, with a planned burst
of 500 events per second. This does not yet justify operating Kafka, NATS,
RabbitMQ, or Redis as a third durable system.

A statement that “jobs use PostgreSQL leases” is not sufficient design. A safe
queue also needs a state machine, atomic enqueue rules, active deduplication,
retry/dead-letter behavior, protection from stale workers, a response to work
arriving while a job is already running, fair scheduling, retention, and clear
delivery semantics.

Two subtle races require explicit treatment:

1. A worker can pause longer than its lease. Another worker then reclaims the
   job, while the old worker later resumes and tries to commit stale output.
2. A `project_trace` job can already be running when a new event for the same
   trace is ingested. Merely deduplicating the second job into the active row can
   lose the required second projection if the first worker loaded events before
   the new event committed.

PostgreSQL `LISTEN/NOTIFY` also cannot be treated as the durable queue. A
listener belongs to a live database session, registration has a startup race,
and notification delivery is a signaling mechanism around committed table data.
The jobs table, rather than a notification, must remain authoritative.

## Decision drivers

- Atomically commit source data and required work in one PostgreSQL transaction.
- Preserve acknowledged work across process restarts and ordinary deploys.
- Support multiple worker processes without holding database transactions open
  during RPC, report rendering, or other external work.
- Prevent an expired, stale worker from committing authoritative state after a
  newer worker owns the job.
- Coalesce bursts without losing work that arrives during an active execution.
- Make retry, exhaustion, and operator recovery visible and testable.
- Keep ingestion latency and database connections protected from background
  workloads.
- Fit the P0 design point and self-hosted two-container deployment.
- Avoid claiming exactly-once execution when crashes make at-least-once the
  honest contract.
- Keep job payloads bounded, internal, versioned, and free of prohibited data.
- Preserve a migration path to a broker if measured scale or independent
  consumer requirements later justify one.

## Options considered

### Option A — PostgreSQL job table with row leases

Domain writes and job insertion share one transaction. Workers atomically claim
eligible rows with `FOR UPDATE SKIP LOCKED`, commit the short claim transaction,
perform work outside it, and finish through a lease-fenced transaction.

This reuses the required durable system, prevents a dual-write gap, supports
horizontal workers, and is sufficient for the planned volume. It requires
careful queue code, indexes, vacuum monitoring, and idempotent handlers.

### Option B — External durable broker in P0

Kafka, NATS JetStream, RabbitMQ, or a similar broker could provide specialized
delivery, consumer groups, and independent scaling. Because events are first
stored in PostgreSQL, reliable publication would still require a transactional
outbox and relay; a direct database-plus-broker write would create an ambiguous
failure window. The extra service, credentials, storage, monitoring, backup,
and recovery behavior are not justified by the current workload.

### Option C — Redis queue

Redis-backed queue libraries provide convenient delayed jobs and retries. Redis
would become another required durable system, while PostgreSQL still owns the
events and projections. It does not eliminate dual-write coordination and adds
data-loss/durability configuration that P0 otherwise does not need.

### Option D — In-memory channels and timers only

Tokio channels and an in-memory priority queue are efficient wake-up and
scheduling tools. Used alone, they lose work on restart and cannot coordinate
separate server roles. They remain useful only as caches or local dispatch
mechanisms over PostgreSQL-owned state.

### Option E — PostgreSQL advisory locks without job rows

Advisory locks can coordinate live sessions but do not themselves persist job
payload, due time, attempts, errors, completion, or dead-letter history. Session
loss releases the lock but does not explain what must be retried. They do not
replace a durable job state machine.

### Option F — A PostgreSQL queue extension or general-purpose job framework

An extension or library could reduce custom queue code. It would add image and
upgrade dependencies and might not support Landfall's atomic event enqueue,
generation-based coalescing, lease fencing, and type-specific domain
transactions. P0 keeps the queue small and explicit; a mature library may be
reconsidered if it satisfies the same invariants without expanding operations.

## Decision

Landfall P0 will use a first-party `work.jobs` PostgreSQL table as its durable
background queue. No external broker, Redis instance, queue extension, or
in-memory-only queue is required.

The delivery contract is **at least once**. Every handler must be idempotent or
must commit its durable effects and job transition atomically behind the current
lease fence. Landfall does not claim exactly-once execution.

Workers claim bounded batches with a short PostgreSQL transaction using
`FOR UPDATE SKIP LOCKED`. The claim creates a time-bounded lease and increments
a monotonic `lease_version`. Every heartbeat, completion, retry, and terminal
transition must match the job ID, worker ID, and lease version and must occur
before the lease expires. A stale worker is rejected even if it later resumes.

Active deduplication uses a partial unique index. Job types that coalesce work
also maintain `requested_generation` and `claimed_generation`; a request that
arrives while generation N is running forces generation N+1 to remain runnable
after N finishes. Deduplication therefore reduces redundant concurrent jobs
without losing new work.

Polling the jobs table is the authoritative P0 wake-up mechanism. P0 will not
depend on `LISTEN/NOTIFY`. A later implementation may use one generic
notification as a best-effort wake-up hint, but correctness, recovery, and
latency bounds must continue to work when every notification is missed.

## Detailed design and boundaries

### Responsibilities

The queue is responsible for:

- durable intent to perform internal background work;
- atomic enqueue with the domain change that requires the work;
- due-time and priority ordering within bounded queue lanes;
- exclusive normal ownership through a renewable lease;
- at-least-once recovery after crash or lease expiry;
- coalescing only where the job type defines safe merge semantics;
- retry scheduling, terminal failure, and auditable manual replay;
- operational status, lag, and failure metrics.

The queue is not responsible for:

- storing immutable customer lifecycle events;
- executing customer Solana transactions;
- guaranteeing exactly-once external side effects;
- providing arbitrary user-defined jobs or code execution;
- replacing the in-memory batching/rate-limit logic used within one worker;
- serving as a public API contract.

### Queue lanes and job registry

One physical table serves four logical lanes:

| Lane | Initial job types | Isolation purpose |
|---|---|---|
| `projection` | `project_trace`, `recompute_metrics` | Keeps accepted telemetry moving into queryable state |
| `observation` | `observe_signature`, `enrich_transaction` | Applies RPC-specific rate limits and batching |
| `report` | `generate_report` | Prevents CPU/memory-heavy rendering from starving ingestion/projection |
| `maintenance` | `delete_trace`, `apply_retention` | Bounds destructive and bulk database work |

`queue_name` is derived from `job_type` by a closed Rust job registry; callers do
not select an arbitrary lane or handler. The registry defines for each job type:

- payload type and payload version;
- queue lane;
- enqueue mode;
- allowed priority range and default;
- lease duration and heartbeat behavior;
- maximum attempts and retry classifier;
- base/cap backoff;
- maximum claim batch and handler concurrency;
- effect idempotency strategy;
- safe payload merge function when coalescing is enabled.

Unknown job types or payload versions fail before insertion. Database rows remain
`TEXT`/`JSONB` for migration flexibility, but only the typed registry constructs
or dispatches them.

### `work.jobs` row

The logical P0 row contains:

| Column | Purpose and invariant |
|---|---|
| `id UUID PRIMARY KEY` | UUIDv7 job identity; stable across attempts |
| `queue_name TEXT NOT NULL` | Closed lane derived from job type |
| `job_type TEXT NOT NULL` | Closed handler identifier |
| `payload_version SMALLINT NOT NULL` | Selects the typed internal payload decoder |
| `payload JSONB NOT NULL` | Bounded, allowlisted internal parameters; never arbitrary request data |
| `dedupe_key TEXT` | Bounded server-generated, fully scoped key; null disables active dedupe |
| `status TEXT NOT NULL` | `ready`, `running`, `retry`, `complete`, or `dead` |
| `priority SMALLINT NOT NULL` | Registry-bounded priority within one lane; larger runs first |
| `run_after TIMESTAMPTZ NOT NULL` | Earliest database time at which work is eligible |
| `attempt_count INTEGER NOT NULL` | Claims for the currently required generation |
| `max_attempts INTEGER NOT NULL` | Registry value copied for auditability |
| `requested_generation BIGINT NOT NULL` | Latest coalesced work generation, initially 1 |
| `claimed_generation BIGINT` | Generation snapshot owned by the current lease |
| `lease_version BIGINT NOT NULL` | Monotonic fencing number, incremented on every claim |
| `locked_by UUID` | Random worker-process instance ID for the current lease |
| `locked_until TIMESTAMPTZ` | Database-time lease expiry |
| `last_error_code TEXT` | Stable sanitized internal code; never raw exception text by default |
| `last_error_detail JSONB` | Optional bounded allowlisted diagnostic fields |
| `created_at`, `updated_at` | Database timestamps |
| `finished_at` | Set only for `complete` or `dead` |
| `replay_of UUID` | Optional link from an operator-created replay to the original dead job |

Check constraints enforce state shape:

- `ready` and `retry` have no lease owner/expiry/claimed generation;
- `running` has all lease fields and a claimed generation;
- `complete` and `dead` have no lease fields and have `finished_at`;
- counters are non-negative, generations are positive and ordered;
- payload/error size limits are enforced before SQL and, where practical, by
  database checks.

Timestamps use PostgreSQL time for eligibility and leases. Worker wall clocks do
not decide whether a lease is valid.

### Active deduplication and enqueue modes

The active dedupe index is conceptually:

```sql
CREATE UNIQUE INDEX jobs_active_dedupe_uq
    ON work.jobs (job_type, dedupe_key)
    WHERE dedupe_key IS NOT NULL
      AND status IN ('ready', 'running', 'retry');
```

Every dedupe key is constructed by trusted code and includes its complete scope,
for example a trace ID and route ID where both matter. It is bounded and is not
accepted directly from an untrusted API.

The job registry assigns one of three enqueue modes:

1. **Coalesce and rerun.** On active-key conflict, increment
   `requested_generation`, apply the job type's safe payload merge, and pull
   `run_after` earlier when appropriate. Used for trace projection and durable
   observation scheduling.
2. **Return existing.** On active-key conflict, return the existing job without
   changing its generation. Used when the target resource already represents
   one idempotent request, such as a report ID.
3. **Always insert.** Use no dedupe key when every work item is intentionally
   distinct.

No generic “last payload wins” behavior exists. A merge function is allowed only
when the job type proves that merging cannot discard required work or broaden
sensitive scope.

For `project_trace`, the dedupe key identifies the canonical trace and the
payload contains only its identity. If a new raw event arrives while projection
generation 7 is running, the enqueue transaction increments
`requested_generation` to 8. The generation-7 worker cannot mark the row
terminal; it returns the row to `ready` for generation 8 after committing or
discarding its generation-7 result according to the projection transaction.

### Atomic enqueue

A job is inserted in the same explicit PostgreSQL transaction as the durable
state that makes it necessary:

```text
BEGIN
  insert/deduplicate immutable events or update domain state
  insert/coalesce required job rows
COMMIT
return 202 or domain success
```

If job insertion, coalescing, or the domain write fails, the whole transaction
rolls back. The collector returns no `202 Accepted` for an event batch whose
projection intent is not durable. There is no after-commit in-memory enqueue and
no direct database-plus-broker dual write.

Downstream work follows the same rule. A projector commits its typed projection,
diagnoses, and any required observation jobs in the same final transaction that
transitions or reschedules its projection job.

Explicit repository/service code performs enqueueing; hidden database triggers
do not create business jobs in P0. This keeps transaction ownership and tests
visible. Database constraints still enforce the queue invariants.

### Claim algorithm

Each worker owns a random `worker_id` for one process lifetime. It claims only
job types assigned to its enabled server role and only within configured
concurrency budgets.

The claim is one short `READ COMMITTED` transaction, conceptually:

```sql
WITH candidate AS (
    SELECT id
      FROM work.jobs
     WHERE queue_name = $1
       AND (
            (status IN ('ready', 'retry') AND run_after <= now())
         OR (status = 'running' AND locked_until <= now())
       )
     ORDER BY priority DESC, run_after ASC, created_at ASC, id ASC
     FOR UPDATE SKIP LOCKED
     LIMIT $2
)
UPDATE work.jobs AS job
   SET status = 'running',
       attempt_count = attempt_count + 1,
       claimed_generation = requested_generation,
       lease_version = lease_version + 1,
       locked_by = $3,
       locked_until = now() + $4,
       updated_at = now()
  FROM candidate
 WHERE job.id = candidate.id
RETURNING job.*;
```

The final SQL may group only job types with the same lease configuration, or
claim separately by registry entry. It must preserve these semantics:

- deterministic ordering is used before `LIMIT`;
- locked rows are skipped rather than blocking unrelated workers;
- expired `running` rows are eligible for reclaim;
- attempt count and lease fence advance atomically with the claim;
- the transaction commits before handler work begins;
- claim batch size is bounded, initially no more than 100.

`SKIP LOCKED` deliberately does not provide a consistent analytical view. It is
used only for competing queue consumers, never for user queries, metrics, or
domain decisions.

### Leases, heartbeats, and fencing

Every claimed job returns `(job_id, worker_id, lease_version,
claimed_generation)`. A mutation by that handler must match all applicable
values. Lease renewal is a conditional update that also requires
`locked_until > now()`. A late heartbeat cannot resurrect an expired lease.

For short jobs, the configured lease exceeds measured p99 handler time with a
safety margin. Longer bounded jobs heartbeat no later than one third of their
lease duration. An observer does not hold one lease across an entire transaction
lifetime: each bounded RPC/checkpoint step persists state and reschedules the
job. Report generation is bounded by the report limits and heartbeats if needed.

Before writing authoritative effects, a handler opens a short final transaction
and locks/verifies its current job row. If the worker ID, lease version,
generation, or unexpired lease does not match, it aborts the write as stale. The
job-row lock is held only while committing the bounded domain effects and job
transition, not during external RPC or rendering.

This fence closes the pause/reclaim race:

```text
worker A claims lease_version 4
worker A pauses; lease expires
worker B reclaims lease_version 5
worker A resumes and tries to commit with version 4
database rejects worker A as stale
```

Graceful shutdown stops new claims first. A handler whose cancellation is known
to be complete may release its lease with the same fence; otherwise the process
lets the lease expire so that two executions are not intentionally overlapped.

### Completion with generation safety

Within the lease-fenced final transaction:

- if `requested_generation = claimed_generation`, successful work transitions
  to `complete`, clears lease fields, and sets `finished_at`;
- if `requested_generation > claimed_generation`, successful work clears the
  lease and returns the row to `ready`, with `run_after = now()` and attempts
  reset for the newly required generation;
- impossible `requested_generation < claimed_generation` is a constraint or
  corruption failure and makes readiness false until investigated.

For a deterministic full-trace projector, the final transaction also prevents a
new event/coalesce transaction from racing past the generation check. Either the
new event commits first and the projector sees a newer generation, or the
projector completes first and the new enqueue creates or advances runnable
work. There is no interval in which a committed event has neither an active job
nor a later generation.

### At-least-once effects and handler rules

A crash can occur after an external operation or expensive computation but
before completion. The reclaimed job will run again. Therefore each handler
must declare one of these effect strategies in the registry and tests:

- **Pure recomputation:** derive and replace state deterministically, as the
  trace projector does.
- **Idempotent database upsert:** use a stable domain/effect key and commit the
  upsert with the fenced job transition.
- **Read-only external call:** duplicate RPC reads are acceptable; persist their
  normalized result idempotently.
- **External idempotency key:** only for a future external write API that
  explicitly supports a stable key derived from the job/effect identity.

P0 handlers must not perform non-idempotent external writes without a separately
reviewed idempotency design. Job ID alone does not magically make an external
provider exactly once.

Examples:

- `project_trace` rebuilds the full projection from retained events and replaces
  typed child rows transactionally;
- `observe_signature` performs bounded read-only RPC polling and persists
  deduplicated observations/checkpoints before rescheduling;
- `generate_report` may compute twice after a crash but publishes one artifact
  for the stable report ID;
- `delete_trace` and `apply_retention` delete by stable scope in bounded,
  repeatable operations.

### Retry and dead-letter policy

On failure, the handler maps the error to a bounded internal classification:

- **transient:** dependency timeout, temporary connection failure, serialization
  retry, or bounded provider rate limit;
- **permanent:** unsupported payload version, invariant violation, prohibited
  data, missing non-recoverable target, or a configured non-retryable response;
- **unknown:** retried conservatively only within the job type's small maximum
  and surfaced for review.

Each claim increments `attempt_count`, including reclaim after lease expiry. If
the current generation fails transiently and attempts remain, the worker sets
`status = 'retry'`, clears the lease, and schedules:

```text
delay = random(0, min(backoff_cap, backoff_base * 2^(attempt_count - 1)))
run_after = database_now + delay
```

This is exponential backoff with full jitter. A bounded provider `Retry-After`
may raise the delay for observer work. Randomness and database time are
injectable in tests.

A permanent failure or exhausted attempt budget moves the job to `dead`, clears
the lease, records sanitized error metadata, and sets `finished_at`. A newer
requested generation takes precedence over a failure of an older generation:
the row returns to `ready` for the new generation with a reset attempt budget,
while retaining bounded diagnostic evidence about the earlier failure.

Dead jobs are never silently dropped or automatically reset. An authenticated
operator replay creates a new job with `replay_of` pointing to the dead row and
an explicit reason in the operational audit/log context. The original terminal
row remains unchanged until retention.

### Polling and optional wake-up hints

P0 workers use bounded polling. After a successful claim they immediately drain
more eligible work up to their concurrency budget. When a lane is empty, idle
poll delay grows with jitter from approximately 100 ms to at most 1 second. The
exact tunable defaults are benchmarked, but the maximum must keep projection lag
within the less-than-5-second p95 target under design load.

An in-memory timer or observer priority queue can avoid needless queries between
known due times, but placing an ID in memory does not claim, remove, or complete
the PostgreSQL job. Restart reconstructs all responsibility from the table.

If `LISTEN/NOTIFY` is added later, the notification means only “poll the table
now”. It contains no job payload, customer identifier, signature, endpoint,
error, or secret. It may be duplicated, coalesced, delayed, or missed. Normal
polling remains enabled, and listener startup follows listen-commit-query order.

### Indexes and bounded table lifecycle

In addition to the primary key and active dedupe index, the initial design has
targeted B-tree indexes for:

- eligible work by `(queue_name, priority DESC, run_after, created_at, id)` over
  `ready`/`retry` rows;
- expired leases by `(queue_name, locked_until)` over `running` rows;
- terminal cleanup by `(status, finished_at)` over `complete`/`dead` rows;
- operator lookup by bounded type/status/time filters.

P0 does not partition the jobs table. Completed rows are deleted in bounded
maintenance batches after a configurable default of 7 days. Dead jobs default
to 30 days so operators can investigate and replay them. Durable report records,
raw events, and projections have their own retention; deleting a completed job
does not delete its domain result.

Autovacuum progress, dead tuples, table/index size, and query plans are monitored
because a frequently updated queue can bloat. The expected active/history queue
footprint remains below 1 GiB at the P0 design point. Retention never deletes
active rows.

### Fairness and resource isolation

Priority is meaningful only within a lane and is chosen by trusted registry
logic, not by API callers. Initial priorities are bounded to a small fixed range.
Each lane has separate concurrency and claim budgets, so a large report or
retention run cannot consume all projector/observer capacity.

Database connection budgets reserve capacity for ingestion and health checks.
Workers use bounded transactions and concurrency; report and maintenance work
must not hold connections while performing CPU-heavy rendering or external I/O.
Observer batching groups compatible route/options only after jobs are due and
claimed, and per-route rate limits remain authoritative.

Strict global FIFO is not guaranteed because priorities, delayed work, locks,
and multiple consumers intentionally reorder execution. Oldest eligible age per
lane is monitored for starvation. Correctness must not depend on FIFO order.

### What this ADR does not decide

This ADR does not finalize:

- exact SQLx repository method names;
- exact per-job lease, heartbeat, attempts, backoff, batch, and concurrency
  values;
- the full internal JSON payload shape for every job version;
- observer RPC batching and rate-limit algorithms;
- report artifact storage;
- operator UI or future RBAC design;
- which external broker would be selected after a future scaling decision.

Those values belong in the typed registry, migrations, support/operations docs,
benchmarks, threat model, or a later ADR while preserving the invariants here.

## Consequences

### Positive

- Event/domain writes and job intent are atomic without a broker relay.
- P0 keeps two required containers and one backup/restore boundary.
- Lease fencing prevents stale workers from committing after reclaim.
- Generation-based coalescing reduces bursts without losing later events.
- Separate lanes and budgets protect projection and ingestion from heavy work.
- Terminal failures and manual replay are observable and auditable.
- The queue can coordinate separate server roles later without redesigning the
  domain protocol.

### Negative

- Landfall owns concurrency-sensitive queue code and its tests.
- Frequent updates and terminal history create vacuum and index maintenance.
- At-least-once execution means some computation and external reads may repeat.
- PostgreSQL outage stops both ingestion durability and background processing.
- PostgreSQL is not a high-throughput event-stream replacement for unlimited
  consumers or long replay histories.
- Narrow lanes and handler registry add more schema/code than a naive jobs table.

### Risks and mitigations

| Risk | Impact | Mitigation or detection |
|---|---|---|
| Stale worker commits after lease expiry | Newer state is overwritten | Monotonic lease version, unexpired lease check, fenced final transaction, pause/reclaim tests |
| New event arrives during projection | Trace remains stale | Requested/claimed generations, active partial uniqueness, transactional completion race tests |
| Handler runs twice after crash | Duplicate effect | Explicit at-least-once contract, per-handler idempotency strategy and effect keys |
| Worker holds a database transaction during RPC/rendering | Lock contention and ingestion latency | Short claim/final transactions; external work between them; transaction-duration metrics |
| High-priority stream starves older work | Unbounded lag | Separate lanes/concurrency, bounded trusted priorities, oldest-ready-age alerts |
| Queue table/index bloat | Slow claims and storage growth | 7/30-day cleanup, bounded delete batches, autovacuum/table-size monitoring, plan benchmarks |
| Retry storm during dependency outage | Database/RPC overload | Full-jitter backoff, lane concurrency caps, rate limits, readiness/status warnings |
| Dead jobs go unnoticed | Permanently stale projections/reports | Per-type dead count, oldest dead age, authenticated inspection/replay workflow |
| Arbitrary payload/error leaks secrets | Sensitive data retained and displayed | Typed payload allowlists, size limits, sanitized errors, payload/log security fixtures |
| Polling creates needless load | Database contention | Bounded idle backoff, small indexed claims, query latency/plan monitoring, optional hint later |
| Notification is mistaken for durable work | Lost jobs after disconnect/race | Polling/table authority invariant; no P0 dependency on notification |
| One PostgreSQL failure stops all durable paths | Ingestion and workers unavailable | No false `202`, readiness false, SDK fail-open buffer, tested DB recovery/backup |

## Security and privacy impact

Jobs are internal control records. Public clients cannot choose `job_type`, SQL,
priority, handler, dedupe key, lease fields, retry limits, or payload version.
Authenticated API actions create a sanitized domain record first; the internal
job normally carries only that record's ID.

Job payloads and error details use a closed allowlist and strict size bounds.
They must not contain private keys, seed phrases, authorization headers, cookies,
RPC credentials, complete credential-bearing URLs, raw signed transactions,
arbitrary event JSON, arbitrary report filters, or unrestricted exceptions and
stack traces. Logs identify jobs by job ID/type/attempt and do not dump payloads.

Database queries are parameterized. Job type and lane select closed code paths;
payload values cannot select table names, SQL fragments, filesystem paths, or
network destinations. Report and deletion handlers reload authorized,
environment-scoped domain records rather than trusting scope embedded in JSON.

Dead-job inspection and replay are authenticated administrative operations and
must honor project/environment scope. P0's authorization model is finalized in
the threat model; this ADR does not claim hosted tenant-grade isolation.

PostgreSQL documentation states that notifications are visible to all database
users. Therefore a future `NOTIFY` payload is generic and contains no Landfall
identifier or sensitive value. P0 sends no queue notification at all.

## Reliability and failure behavior

The queue provides durable at-least-once work after the enqueue transaction
commits. It does not promise that every handler begins immediately or runs only
once.

Failure boundaries are explicit:

| Failure point | Durable result | Recovery |
|---|---|---|
| Before enqueue transaction commits | Neither domain change nor job is accepted | Caller retries idempotently |
| After commit, before response | Domain change and job both exist | Caller replay deduplicates; worker proceeds |
| Worker dies before claim commit | Job remains eligible/unclaimed | Any worker claims it |
| Worker dies after claim | Running row retains lease | Another worker reclaims after expiry |
| Worker loses DB connectivity during work | It cannot renew or commit fenced effects | It abandons; lease expires and work repeats |
| Worker commits effects, then response is lost | Effects and job transition are already atomic | No rerun unless a newer generation exists |
| External read succeeds, process dies before persistence | No durable result | Read may repeat safely |
| Permanent handler failure | Job becomes `dead` with sanitized reason | Alert, fix, authenticated replay to a new job |
| PostgreSQL unavailable | No enqueue, claim, completion, or true readiness | API returns `503`; workers back off and reconnect |

Serialization/deadlock errors in short queue/domain transactions are retried at
the transaction boundary with a small bounded policy. A job handler's own
attempt count is not incremented for a claim transaction that never committed.

Queue backlog does not by itself permit dropping projection, deletion, or
observation responsibility. The service reports degraded lag. It rejects an
ingestion batch only when it cannot atomically persist the events and required
jobs, not merely to make queue metrics look healthy.

## Performance and capacity impact

At the P0 design point, approximately 50,000 traces/day create a similar order
of projection lifecycles; event bursts coalesce by trace. Observation checks are
scheduled durably but RPC reads are batched, and completed job rows are short
lived. This is substantially below the event-row volume and is expected to keep
the queue below 1 GiB with retention.

The queue benchmark must cover:

- concurrent enqueue/coalesce at the 500-events/second ingestion burst;
- at least four competing workers claiming eligible batches;
- active running rows and expired lease reclaim;
- a realistic mix of projection, observation, report, and maintenance rows;
- seven days of completed-history cardinality before cleanup;
- enqueue latency inside the 100-event ingestion transaction;
- claim/finalization p50/p95 and PostgreSQL query plans;
- projection lag p95 under five seconds on reference hardware.

The first implementation uses claim batches no larger than 100 and bounded
worker concurrency. Increasing a batch is not free: it holds row locks longer
and may lease more work than a worker can finish. Measurements choose smaller
per-lane defaults.

Reconsider the queue architecture when measured queue activity materially
causes ingestion p95 to exceed 250 ms, projection lag exceeds 30 seconds under
normal load after database/index tuning, active plus retained job state exceeds
the planned footprint, or work requires independent durable fan-out/replay that
the row model cannot provide.

## Operational impact

PostgreSQL remains the single durable backup/restore dependency for events,
projections, and queued work. Restoring a consistent database snapshot restores
job state with the corresponding domain data. In-flight leases from a restored
or stopped process expire and are reclaimed; worker IDs are never reused across
process starts.

The system status and structured metrics expose, per lane and job type:

- ready/retry/running/dead counts;
- oldest eligible age and scheduled future count;
- enqueue, claim, completion, retry, and dead rates;
- handler duration and attempt distribution;
- expired-lease reclaims and rejected stale commits;
- dedupe conflicts and generation reruns;
- poll/claim query latency and rows scanned;
- jobs retained and table/index/dead-tuple size.

Alerts or prominent health warnings cover projection lag, any dead projection or
deletion job, repeated lease expiry, retry growth, stalled lanes, and cleanup
failure. A dead report job is visible to its requester and operations but does
not make ingestion unready.

Migrations are append-only after release. New job payload versions are deployed
handler-first, then producers; workers must not claim a version they cannot
decode. Removing a handler waits until no active/retained job requires it or an
explicit migration/dead-letter decision is recorded.

## Verification

ADR-007 is implemented only when all of the following are true:

- Concurrent domain/event writes and enqueue either commit together or both
  roll back.
- A `202 Accepted` ingestion response is impossible when its required
  `project_trace` job intent is absent.
- Concurrent enqueue of the same active dedupe key creates one active row and
  the expected generation count.
- An event committed while projection is running always leaves a newer runnable
  generation or a projection that includes it.
- Competing workers using `SKIP LOCKED` never hold the same current lease under
  normal claims and do not block on already leased rows.
- A paused worker with lease version N is unable to commit after version N+1 is
  claimed, even if N's computation later succeeds.
- Lease expiry, heartbeat, reclaim, graceful shutdown, and database-time behavior
  pass deterministic fake-clock/integration tests.
- Every handler has a documented and tested effect-idempotency strategy.
- Crash injection covers before claim commit, after claim, after external work,
  during final domain transaction, and after final commit.
- Transient, permanent, unknown, exhausted, newer-generation, and provider
  rate-limit failures take their specified retry/dead transitions.
- Manual replay creates a linked new job without mutating the dead original.
- Worker transactions remain short and no RPC/render operation holds a database
  transaction or row lock.
- Typed payload/error fixtures reject unknown versions, excessive size,
  credentials, raw signed bytes, unsafe URLs, arbitrary SQL/path values, and
  unrestricted errors.
- Polling alone processes jobs and meets lag targets with notifications entirely
  disabled.
- Queue query-plan and load benchmarks pass the capacity cases in this ADR and
  keep ingestion/projection latency within their P0 targets.
- Retention deletes only eligible terminal rows in bounded batches and leaves
  domain results and active jobs intact.
- Backup/restore and forced-worker-termination tests prove that committed work
  is recovered.

## Rollout and migration

P0 introduces the queue before the ingestion API can return `202`. Rollout order
is:

1. Create the `work` schema, jobs table, constraints, indexes, and repository.
2. Implement the typed registry and transactional enqueue/coalesce paths.
3. Implement claim, lease fencing, heartbeat, completion, retry, dead, and
   retention behavior with integration tests.
4. Implement one deterministic projector handler end to end.
5. Enable ingestion only after event-plus-job atomicity tests pass.
6. Add observer, report, deletion, and retention handlers one at a time with
   their idempotency tests and lane limits.

There is no existing production queue to migrate. During ordinary deployments,
old and new binaries may overlap only when they understand the same active job
payload versions and lease/state semantics. Otherwise workers are drained,
leases expire or complete, migrations run, and the compatible worker version
starts before new producers enqueue a new payload version.

If a broker later becomes necessary, PostgreSQL jobs first become a
transactional outbox: a relay publishes stable job/effect IDs, records broker
publication idempotently, and consumers preserve the same handler idempotency
contract. Job types move separately. Landfall never introduces a direct
database-plus-broker dual write and never deletes the PostgreSQL authority until
replay, cutover, rollback, and failure tests prove the replacement.

## Reconsideration triggers

- More than 10 million accepted events/day or a corresponding measured job rate
  exceeds the tuned reference profile.
- Projection lag exceeds 30 seconds under normal load after handler, index,
  vacuum, connection, and database tuning.
- Queue operations materially push 100-event ingestion p95 above 250 ms.
- Queue/history size or update bloat cannot remain within the PostgreSQL storage
  and maintenance budget.
- Multiple independently deployed consumers require durable fan-out and replay,
  rather than one owned handler per job type.
- Hosted regional or tenant-isolation requirements need independent failure,
  quota, or data-placement boundaries.
- An external non-idempotent workflow requires broker capabilities and an
  explicit end-to-end idempotency design.
- A mature PostgreSQL queue library/extension proves it satisfies every fencing,
  generation, atomicity, security, and operational invariant with lower cost.
- PostgreSQL backup/recovery objectives become incompatible with the required
  work-delivery objective.

## References

- [Product requirements: processing model](../product-requirements-document.md#188-processing-model)
- [Product requirements: reliability](../product-requirements-document.md#211-reliability)
- [System design: processing consistency](../system-design.md#78-processing-consistency)
- [System design: job queue](../system-design.md#810-job-queue)
- [System design: capacity estimation](../system-design.md#5-capacity-estimation)
- [Technical implementation plan: PostgreSQL persistence and job infrastructure](../technical-implementation-plan.md#12-phase-5--postgresql-persistence-and-job-infrastructure)
- [PostgreSQL 18 `SELECT` locking clauses](https://www.postgresql.org/docs/18/sql-select.html#SQL-FOR-UPDATE-SHARE)
- [PostgreSQL 18 constraints and partial uniqueness](https://www.postgresql.org/docs/18/ddl-constraints.html#DDL-CONSTRAINTS-UNIQUE-CONSTRAINTS)
- [PostgreSQL 18 `LISTEN`](https://www.postgresql.org/docs/18/sql-listen.html)
- [PostgreSQL 18 `NOTIFY`](https://www.postgresql.org/docs/18/sql-notify.html)
- [ADR-001: modular monolith and two-container topology](001-modular-monolith-and-two-container-topology.md)
- [ADR-002: immutable events and relational projections](002-immutable-event-inputs-and-relational-projections.md)
- [ADR-005: event contract and code generation](005-json-schema-event-contract-and-code-generation.md)
