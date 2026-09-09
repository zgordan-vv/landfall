# Shared protocol fixtures

`v1/manifest.json` is the registry for payloads shared by JSON Schema, Rust, and
TypeScript/Node verification. Fixture payloads contain only wire JSON, so they
can later be sent unchanged through the CLI and ingestion API.

- `valid/` contains accepted payloads. Every accepted payload must survive a
  Rust deserialize/serialize round trip without changing its JSON value.
- `valid/incidents/` contains the twelve golden situations named in the
  implementation plan. These are protocol inputs for the Phase 3 reducer and
  are not yet assertions about derived diagnoses.
- `invalid/` contains one intentional defect per payload. The manifest records
  whether structural JSON Schema validation or typed semantic validation owns
  the rejection, plus the stable error category expected by callers.

Privacy and security payloads live in a separate corpus so each v1 invalid
fixture diagnoses one protocol violation rather than mixing validation and
privacy failures.

## Privacy corpus

`privacy/manifest.json` separates two different controls:

- `reject` inputs contain a prohibited field or violate a hard schema bound and
  must never reach typed persistence;
- `redact` inputs are structurally valid because secrets were smuggled inside an
  allowlisted bounded text field. Their paired expected documents replace the
  complete affected field with `[REDACTED]` and change nothing else.

The fixture checker proves these expectations now. SDK and collector privacy
implementations must later transform every `redact` input into its paired output
before persistence; the corpus does not pretend that JSON Schema alone scans
the contents of allowed strings.
