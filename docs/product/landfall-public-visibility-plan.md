# Landfall public visibility plan

Goal: make Landfall visible as an early Solana infrastructure product and show
the engineering work behind it as a real, usable product.

The first audience is people who may need, recommend, or evaluate Solana
engineering work: founders, CTOs, protocol/backend engineers, Web3 agencies,
technical recruiters, and teams debugging real transaction flows.

## Positioning

Short description:

> Landfall is Solana transaction observability: it shows whether a transaction
> was signed, submitted, observed on-chain, executed successfully, or still lacks
> evidence.

Product angle:

> Landfall helps Solana teams debug transaction uncertainty with lifecycle
> evidence instead of guessing from explorer visibility or isolated RPC
> responses.

Engineering angle:

> Built with Rust, PostgreSQL, TypeScript, React, Docker, encrypted private RPC
> routes, monitoring, backup runbooks, and a live public demo.

Demo:

<https://landfall.codehummus.com/#overview>

GitHub:

<https://github.com/zgordan-vv/landfall>

## LinkedIn profile update

Headline:

> Solana / Rust / TypeScript developer | Building Landfall: transaction
> observability for Solana apps

Featured link text:

> Landfall live demo: Solana transaction observability

About snippet:

> I build backend-heavy Web3 products with Rust, TypeScript, PostgreSQL, and
> Docker. I am currently building Landfall, a Solana transaction observability
> system that tracks lifecycle evidence from signing through on-chain
> observation and reports what succeeded, failed, or remains unknown.

## First week posts

### Post 1: Product announcement

I am building Landfall: Solana transaction observability.

It tracks lifecycle evidence around a transaction:

- signed
- submitted
- observed on-chain
- execution result
- missing evidence

The point is simple: a block explorer can show what landed, but it cannot always
explain what happened before inclusion.

Stack: Rust, Axum, PostgreSQL, SQLx, TypeScript SDK, React, Docker, Prometheus.

Live demo:
https://landfall.codehummus.com/#overview

GitHub:
https://github.com/zgordan-vv/landfall

### Post 2: Technical lesson

One thing Landfall makes explicit:

"No transaction in explorer" is not a diagnosis.

It can mean:

- it was signed but never submitted;
- the RPC accepted it but it expired;
- it landed on a fork that did not finalize;
- the app missed the confirmation path;
- the observer has not seen enough evidence yet.

That is why Landfall separates lifecycle states instead of collapsing everything
into success/failure.

Demo:
https://landfall.codehummus.com/#traces

### Post 3: Engineering depth

Landfall is not just a dashboard over Solana RPC.

The system includes:

- immutable lifecycle events;
- PostgreSQL projections;
- deduplication;
- token-scoped project access;
- encrypted private RPC routes;
- Docker deployment;
- monitoring and backup runbooks;
- public demo mode.

The goal is to make transaction uncertainty visible and explainable for Solana
applications.

Demo:
https://landfall.codehummus.com/#support

## Threads/X shorter versions

### Short 1

I am building Landfall: Solana transaction observability.

It shows whether a transaction was signed, submitted, observed on-chain,
executed, or still unknown because evidence is missing.

Live demo:
https://landfall.codehummus.com/#overview

### Short 2

"Not found in explorer" is not a diagnosis.

A Solana transaction may be signed but not submitted, accepted by RPC but
expired, missed by confirmation logic, or simply lack enough evidence.

Landfall makes those lifecycle gaps visible.

### Short 3

Landfall is built as a real product surface:

Rust + PostgreSQL + TypeScript SDK + React + Docker + monitoring + docs + live
demo.

It is focused on Solana transaction observability and missing lifecycle
evidence.

Demo:
https://landfall.codehummus.com/#overview

## CodeHummus blog topics

### Blog 1

Title: What actually happens to a Solana transaction before it appears in an
explorer?

Purpose: explain signed, submitted, landed, finalized, failed, and unknown in
plain language. End with Landfall as the product example.

### Blog 2

Title: Why "not found in explorer" is not enough to debug a Solana transaction

Purpose: explain missing evidence, RPC ambiguity, expiration, and confirmation
gaps. Link to Landfall trace detail.

### Blog 3

Title: Building Solana transaction observability with Rust and TypeScript

Purpose: explain the architecture, tradeoffs, docs, deployment, monitoring, and
why lifecycle evidence matters for real Solana applications.

## Posting cadence

Week 1:

- update LinkedIn headline and featured demo link;
- publish Post 1 on LinkedIn;
- publish Short 1 on Threads/X;
- write Blog 1 on CodeHummus.

Week 2:

- publish Post 2 on LinkedIn;
- publish Short 2 on Threads/X;
- reply to relevant Solana/RPC/debugging discussions;
- write Blog 2 outline.

Week 3:

- publish Post 3 on LinkedIn;
- publish Short 3 on Threads/X;
- add blog links to LinkedIn featured section and relevant proposals.

## Tracking

Track manually:

- post URL;
- channel;
- date;
- views;
- reactions;
- relevant comments;
- profile visits;
- demo clicks if available;
- inbound messages;
- technical conversations;
- interview, contract, pilot, or partnership opportunities.
