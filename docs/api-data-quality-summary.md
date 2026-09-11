# Data-quality summary

`landfall_server::data_quality_summary::summarize_data_quality` aggregates the
versioned core assessments used by the read API. It reports:

- `assessments` — number of traces assessed;
- `grades` — stable A/B/C/D/F distribution, including zero-count grades;
- `gaps` — deterministic counts of each distinct finding key;
- `average_score` — integer floor average, or `null` when the selection is empty;
- `definition_version` — the core scoring rubric version.

Gaps are counted per assessment, not per duplicate evidence event. This keeps
the summary useful for cohort analysis and prevents a noisy trace from
dominating the result. Empty selections remain explicit and never imply good
quality.
