# Fault-injection matrix

`fault_injection::expected_outcome` is the deterministic contract for chaos
checks. Collector and observer throttling remain retryable; database or disk
failures halt writes; projector/report crashes rehydrate durable jobs; corrupt
RPC responses are quarantined; and retention failures remain dry-run-only.
Integration tests can inject each `FailureKind` and assert the corresponding
non-destructive outcome.
