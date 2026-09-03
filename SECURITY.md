# Security policy

Landfall handles operational data about financial transactions. Please report
suspected vulnerabilities privately and do not include secrets, private keys,
seed phrases, production transaction bytes, customer telemetry, or working
exploit payloads in a public issue.

## Supported versions

Landfall is pre-release software and does not yet make production-readiness
claims.

| Version | Security support |
|---|---|
| Current `main` branch | Best effort |
| Older commits or forks | Not supported |

A versioned support policy will replace this table before the first production
release.

## Reporting a vulnerability

Use the repository's private **Report a vulnerability** flow under the GitHub
Security tab. If that flow is unavailable, contact the repository owner through
their GitHub profile to request a private channel; do not send vulnerability
details in the initial public message.

Include only sanitized information needed to reproduce and assess the issue:

- affected revision or version;
- affected component and configuration;
- impact and required attacker access;
- minimal reproduction steps using synthetic data;
- suggested mitigation, if known.

You can expect acknowledgement within five business days and an initial
assessment within ten business days. Remediation and disclosure timing depend
on severity and release status. Please allow a coordinated fix before public
disclosure.

## Scope

In scope are Landfall-owned code and defaults affecting authentication,
authorization, ingestion, privacy modes, stored telemetry, report exports,
secret handling, RPC egress, dependency integrity, and deployment isolation.

Wallet, signer, seed phrase, Solana program, provider, host, database
administrator, and third-party dependency vulnerabilities should normally be
reported to their respective owners unless Landfall introduces or amplifies the
issue. The detailed trust boundaries are documented in
[`docs/threat-model.md`](docs/threat-model.md).

There is currently no bug bounty or guarantee of payment. Good-faith reports
that respect user data and avoid service disruption are welcome.
