# Diagnostics and recommendations guide

Landfall findings are deterministic claims over retained evidence, not
AI-generated explanations. Each finding has a stable rule ID, rule-set version,
claim key, certainty, and source-event anchor.

- **Confirmed** means direct evidence supports the claim (for example an
  observed execution error or structured RPC rejection).
- **Probable** means a bounded risk signal is present but causality is not
  proven (for example signing delay or route degradation).
- **Unknown** means required evidence is missing; it is never presented as a
  failure.

Recommendations are advisory mappings from findings to actions. They preserve
the finding and evidence IDs and never sign, submit, retry, or mutate a
customer transaction. See the complete [diagnostic rule catalog](diagnostic-rule-catalog.md).
