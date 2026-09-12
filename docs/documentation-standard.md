# Documentation quality standard

Every public document must identify its audience, purpose, prerequisites,
commands or examples, expected evidence, failure/limitation cases, and links to
the authoritative contract. A one-paragraph design note is not a user guide.

Classify each file as one of: user guide, developer guide, API reference,
operations runbook, security policy, benchmark method, ADR, or internal note.
Use explicit status labels for placeholders and fixture-backed behavior. Keep
examples safe to copy: no real credentials, private keys, customer data, or
destructive command without a confirmation guard.

Review checklist:

- Can a new reader complete the described task from a clean checkout?
- Are inputs, outputs, errors, and evidence visible?
- Are production claims separated from fixture/unit-test evidence?
- Do relative links resolve after documentation moves?
- Is the source of truth (schema, OpenAPI, code, or ADR) named?
