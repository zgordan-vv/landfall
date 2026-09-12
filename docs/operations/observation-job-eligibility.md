# Observation job eligibility

Migration `0009` registers the `observe_trace` job type. The enqueue helper
requires an explicit eligibility decision and uses the trace ID as its active
dedupe key. Ineligible traces produce no database write; repeated eligible
requests share one ready/running job.
