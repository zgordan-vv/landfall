# Diagnostic history

Every ingest that touches a trace re-runs the deterministic rule layers over the
trace's full event snapshot. `record_trace_evaluation` writes the result in one
transaction:

- one `reporting.diagnostic_evaluations` row (snapshot size and evaluation time);
- the diagnostics and recommendations it produced, inserted append-only and
  duplicate-safe together with their evidence links;
- membership rows (`diagnostic_evaluation_diagnostics`,
  `diagnostic_evaluation_recommendations`) naming exactly what that evaluation
  produced.

Readers (`/v1/traces/{id}/diagnostics`, `/v1/traces/{id}/recommendations` and
their `/demo/v1` equivalents) serve only the membership of the newest evaluation:
the one with the largest input snapshot, then the latest evaluation time. A
finding computed from a partial snapshot (for example missing observer coverage
before the observer reported) is therefore withdrawn once the evidence arrives,
yet stays in `reporting.diagnostics` for audit and comparison. Because raw events
are append-only, a larger snapshot is never less complete, so an evaluation that
raced on a smaller snapshot cannot replace a newer one.

Diagnostic row ids are derived from the trace, the anchoring evidence event, the
rule, the rule-set version, the claim, the certainty and the unknown reason. The
same claim over the same evidence reuses its row; a different claim anchored on
the same event (for example two unknown reasons) gets its own row. Recommendation
ids stay anchored on the evidence event so dispositions remain attached across
re-evaluations.

`RULE-UNKNOWN-001` rows store `unknown_reason` (for example
`missing_simulation_evidence`), which the diagnostics API returns so clients can
explain the specific gap.

Rows written before evaluations were tracked (migration 0016) were grouped into
one legacy evaluation per trace, which any later re-evaluation supersedes.
