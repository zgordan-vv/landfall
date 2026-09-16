# Customer onboarding and support

This guide explains how a new Landfall user moves from a public demo to a live
workspace without needing billing.

## Who this is for

Use this flow for:

- an evaluator who wants to understand the product without secrets;
- a project administrator who is configuring a real application;
- a developer who needs SDK credentials and private RPC observation;
- a support handoff where the user needs to share diagnostics safely.

## Demo mode

The public demo is read-only. It lets visitors inspect real Solana devnet traces
without seeing customer credentials, private RPC URLs, ingestion tokens, or
payment controls.

Demo mode is the right first screen for Upwork interviewers and product
evaluators. They can open Overview, Traces, Comparison, and Support without
creating an account or pasting a token.

## Live workspace setup

A live workspace needs these objects:

1. Project
2. Dashboard token
3. Environment
4. Private RPC route
5. SDK token

A project represents one application or product. It owns that product's traces,
environments, private RPC routes, access tokens, reports, and x402 audit records.

The dashboard token is for reading Overview, Traces, and Comparison. The
administrator token is for setup and sensitive control-plane actions: creating
routes, generating/revoking tokens, reading x402 audit records, and creating
reports.

## Empty states

The dashboard should explain what to do when data is missing:

- no traces: install the SDK or run the demo ingestion flow;
- no comparison: create traces in at least two environments in the same project;
- no x402 records: run the policy flow in safe mode before testing real payment
  settlement;
- no reports: create a report after traces exist.

## Paid pilot payment

Landfall does not provide card checkout yet. The first paid pilots should use
manual payment outside the product while Landfall remains the delivery system
for instrumentation, diagnosis, and reports.

Before accepting payment, agree on:

- transaction flow and Solana environment;
- audit window and expected traffic volume;
- private RPC or public RPC boundary;
- report format and support channel;
- fixed price and payment method.

Acceptable manual channels include an Upwork contract, invoice, bank transfer,
Wise, Payoneer, or crypto by explicit agreement. Do not store card data, bank
credentials, payment credentials, or customer secrets in Landfall.

After payment, create the Landfall project, configure the environment and RPC
route, issue SDK/dashboard tokens, run the audit, and export the lifecycle
evidence report.

## Safe support handoff

When asking for help, users may share:

- current dashboard page URL;
- project ID, environment name, and cluster;
- trace ID, transaction signature, and visible diagnosis text;
- whether the issue started after first setup, RPC provider change, SDK release,
  or payment-policy test.

Users must not share bearer tokens, private RPC URLs, wallet seed phrases, raw
signed transaction bytes, or production customer data in support messages.
