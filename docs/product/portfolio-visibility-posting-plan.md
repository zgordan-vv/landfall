# Portfolio visibility posting plan

Goal: make Landfall visible as proof that the builder can design, implement,
secure, deploy, and explain a serious Solana developer project.

This is not the customer-discovery campaign yet. The first audience is people
who can hire or recommend a Solana developer: founders, CTOs, engineering
leads, Web3 agencies, and technical recruiters.

## Positioning

Short description:

> Landfall is a Solana transaction observability demo: it shows whether a
> transaction was signed, submitted, observed on-chain, executed successfully,
> or still lacks evidence.

Portfolio angle:

> I built Landfall as a production-style portfolio case: Rust backend,
> PostgreSQL event storage, TypeScript SDK, React dashboard, Docker deployment,
> monitoring, backups, security notes, and a live public demo.

Demo link:

<https://landfall.codehummus.com/#overview>

GitHub:

<https://github.com/zgordan-vv/landfall>

## LinkedIn profile update

Headline:

> Solana / Rust / TypeScript developer | Building Landfall: transaction
> observability for Solana apps

Featured link text:

> Live demo: Landfall transaction observability

About snippet:

> I build backend-heavy Web3 products with Rust, TypeScript, PostgreSQL, and
> Docker. My current portfolio project is Landfall, a Solana transaction
> observability system that tracks lifecycle evidence from signing through
> on-chain observation and reports what succeeded, failed, or remains unknown.

## First week posts

### Post 1: Build announcement

I built a Solana transaction observability project for my portfolio.

Landfall tracks lifecycle evidence around a transaction:

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

One thing I learned while building a Solana observability project:

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

### Post 3: Architecture proof

I wanted my Solana portfolio project to show more than UI screens, so I built
Landfall around backend/product engineering concerns:

- immutable lifecycle events;
- PostgreSQL projections;
- deduplication;
- token-scoped project access;
- private RPC route encryption;
- Docker deployment;
- monitoring and backup runbooks;
- public demo mode.

The goal was to make something I can explain in an interview like a real system,
not just a mockup.

Demo:
https://landfall.codehummus.com/#support

## Threads/X shorter versions

### Short 1

Сделал портфолио-проект для Solana dev work: Landfall.

Это transaction observability: видно, была ли транзакция signed, submitted,
observed on-chain, executed или всё ещё unknown из-за нехватки evidence.

Live demo:
https://landfall.codehummus.com/#overview

### Short 2

Explorer показывает, что попало on-chain.

Но если транзакции там нет, это ещё не диагноз.

Она могла не отправиться, истечь, потеряться в confirmation flow или просто не
иметь enough evidence. Landfall как раз показывает эти lifecycle gaps.

### Short 3

Для портфолио я хотел не “ещё один React dashboard”, а систему, которую можно
объяснить как production backend:

Rust + PostgreSQL + TypeScript SDK + React + Docker + monitoring + docs +
public demo.

Landfall:
https://landfall.codehummus.com/#overview

## CodeHummus blog topics

### Blog 1

Title: What actually happens to a Solana transaction before it appears in an explorer?

Purpose: explain signed, submitted, landed, finalized, failed, and unknown in
plain language. End with Landfall as a portfolio demo.

### Blog 2

Title: Why "not found in explorer" is not enough to debug a Solana transaction

Purpose: explain missing evidence, RPC ambiguity, expiration, and confirmation
gaps. Link to Landfall trace detail.

### Blog 3

Title: Building a production-style Solana portfolio project with Rust and TypeScript

Purpose: explain the architecture, tradeoffs, docs, deployment, monitoring, and
why the project is useful as proof of engineering ability.

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
- add blog links to Upwork proposals and LinkedIn featured section.

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
- interview or contract opportunities.
