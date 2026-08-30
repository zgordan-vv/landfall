# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for Landfall.
An ADR records one consequential engineering decision, the constraints that
shaped it, the alternatives considered, and the consequences the team accepts.

The product requirements document describes what Landfall must do. The system
design describes how the parts fit together. ADRs explain why a specific design
choice was made and preserve that reasoning as the implementation evolves.

## Index

| ADR | Title | Status | Date | Supersedes |
|---|---|---|---|---|
| [000](000-template.md) | ADR template | Template | — | — |
| [001](001-modular-monolith-and-two-container-topology.md) | Modular monolith and two-container topology | Accepted | 2026-08-30 | — |
| [002](002-immutable-event-inputs-and-relational-projections.md) | Immutable event inputs and relational projections | Accepted | 2026-08-30 | — |
| [003](003-business-action-trace-attempt-event-and-alias-identifiers.md) | Business-action, trace, attempt, event, and alias identifiers | Accepted | 2026-08-30 | — |
| [004](004-privacy-modes-and-signed-byte-fingerprints.md) | Privacy modes and signed-byte fingerprints | Accepted | 2026-08-30 | — |

Planned records:

| ADR | Working title |
|---|---|
| 005 | JSON Schema event contract and code-generation direction |
| 006 | Solana Kit first adapter and compatibility roadmap |
| 007 | PostgreSQL job queue instead of an external broker |
| 008 | Code-first OpenAPI with Utoipa |

A planned record is not a decision. It becomes authoritative only after its ADR
file exists and its status is `Accepted`.

## Status lifecycle

Every ADR has exactly one of these statuses:

- `Proposed` — written for review but not yet authoritative;
- `Accepted` — approved and currently authoritative;
- `Rejected` — considered but deliberately not adopted;
- `Deprecated` — retained for history but no longer recommended;
- `Superseded by ADR-NNN` — replaced by a newer accepted decision.

An accepted ADR is historical evidence. Do not rewrite it to make a later design
look inevitable. For a material change, create a new ADR, link both records, and
mark the old record as superseded. Typographical fixes and link repairs are
allowed when they do not change the recorded decision.

## Creating an ADR

1. Copy `000-template.md` to the next unused three-digit number, followed by a
   short kebab-case title, for example `009-report-artifact-storage.md`.
2. Replace every instructional placeholder and remove sections that genuinely
   do not apply.
3. Set the initial status to `Proposed` and use an ISO 8601 date (`YYYY-MM-DD`).
4. Add the record to the index in this file.
5. Review the decision together with affected contracts, schemas, threat model,
   tests, and operational documentation.
6. Change the status to `Accepted` only when the decision has been approved.

## Review standard

An ADR is ready for acceptance when a new engineer can answer all of these
questions from the record alone:

- What concrete problem or uncertainty forced a decision?
- Which product and system constraints matter?
- Which realistic alternatives were considered?
- Why does the chosen option fit Landfall better now?
- What benefits, costs, risks, and future migration pressure are accepted?
- How does the decision affect security, privacy, reliability, performance, and
  operability?
- How will tests or measurable evidence show that the decision works?
- Which documents, APIs, schemas, or components are affected?

## Scope

Create ADRs for choices that are expensive to reverse, affect multiple
components, define a public contract, or materially change security and
operations. Routine implementation details that are local and easily reversible
belong in code, tests, or normal documentation instead.
