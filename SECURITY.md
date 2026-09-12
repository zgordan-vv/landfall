# Security policy

Landfall handles operational data about financial transactions. Please report
suspected vulnerabilities privately and do not include secrets, private keys,
seed phrases, production transaction bytes, customer telemetry, or working
exploit payloads in a public issue.

## Supported versions

Landfall is pre-release software and does not make production-readiness claims.
The current `main` branch receives best-effort security fixes; older commits
and forks are not supported.

## Reporting a vulnerability

Do not open a public issue for a suspected credential leak, authentication
bypass, data-exposure bug, or denial-of-service weakness. Use a private GitHub
Security Advisory for the repository, or contact the project maintainer
through the private channel configured for the deployment. Do not include live
tokens, private keys, seed phrases, production RPC URLs, or customer data in a
report; replace them with synthetic fixtures.

Please include the affected commit/version, deployment mode, reproduction
steps, expected versus observed behavior, impact, and a minimal redacted log or
fixture. Reports that cannot be reproduced are still triaged, but may require
additional evidence.

## Response process

The maintainer acknowledges receipt, confirms scope, reproduces the issue in an
isolated fixture deployment, assigns severity, and prepares a fix or mitigation.
Disclosure timing is coordinated with the reporter; no guaranteed SLA or bug
bounty is offered for this portfolio project.

Acknowledgement is targeted within five business days and initial assessment
within ten business days. Remediation and disclosure timing depend on severity
and release status.

## Scope

In scope are Landfall-owned authentication, authorization, ingestion, privacy,
stored telemetry, report exports, secret handling, RPC egress, dependency
integrity, and deployment isolation. Wallets, signers, Solana programs,
providers, hosts, DBAs, and third-party dependencies should normally be
reported to their respective owners unless Landfall introduces or amplifies the
issue. See [`docs/threat-model.md`](docs/threat-model.md) for trust boundaries.

## Security limitations

Landfall is an observability and evidence platform, not a custody, signing, or
transaction-submission service. It does not protect a compromised host,
malicious administrator, compromised reverse proxy, stolen deployment secret,
or a dishonest blockchain/RPC provider. Production deployments still require
TLS termination, private PostgreSQL networking, least-privilege credentials,
backups, patching, and operator review of the release security gate.

The P0 implementation provides bounded processing, privacy redaction,
hash-only bearer records, and explicit uncertainty. It does not claim
high-availability guarantees, formal verification, statistical significance of
comparisons, or complete protection from volumetric network denial of service.
