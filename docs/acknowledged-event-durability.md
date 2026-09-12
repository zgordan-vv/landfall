# Acknowledged-event durability check

An ingestion acknowledgement is valid only after the event/dedup transaction
commits. `find_durability_gaps` compares the acknowledged event IDs with the
durable event registry and returns deterministic gaps. Any non-empty result is
a failed resilience check and must trigger investigation/replay; it is never
converted into a successful metric.
