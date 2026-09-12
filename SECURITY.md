# Security policy

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
