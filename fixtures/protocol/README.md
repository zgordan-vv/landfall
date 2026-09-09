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

Task 2.9 adds hostile redaction/security payloads separately. Keeping them out
of this initial corpus makes each current invalid fixture diagnose one protocol
contract violation rather than a mixture of validation and privacy failures.
