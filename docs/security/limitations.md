# Security and product limitations

Landfall is a self-hosted P0 reference implementation, not a hosted
multi-tenant service. The operator owns host, PostgreSQL, reverse proxy,
backups, encryption, patching, and access control.

The system does not hold keys, sign or submit transactions, guarantee chain
inclusion, or provide cryptographic erasure from independent backups. Observer
gaps remain Unknown, and a recommendation is advisory rather than an automated
transaction action.

Public exposure requires TLS termination and reviewed proxy limits. Before a
pilot, complete the [threat model](threat-model.md), [TLS runbook](tls-reverse-proxy-runbook.md),
backup restore test, dependency updates, and secret rotation procedure.
