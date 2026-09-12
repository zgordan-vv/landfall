# Acknowledged-event durability check

## Failure being detected

The dangerous failure boundary is an API response that says an event was
accepted while the event/dedup transaction was rolled back, lost, or never
written. Such a response would make downstream metrics and replay decisions
trust data that does not exist.

## Check procedure

1. Capture the event IDs for which ingestion returned an acknowledgement.
2. Read the durable event registry after the transaction boundary.
3. Call `find_durability_gaps(acknowledged, durable)`.
4. Treat every returned `DurabilityGap` as a failed check; retain the IDs for
   replay/investigation and do not count them as successful ingestion.

The function sorts gaps by event ID, making CI output and incident diffs
stable. An empty result means only that the selected acknowledged IDs were
present in the registry; it does not prove projection completion.

## Test

Run `cargo test -p landfall-server acknowledged_events`. The test covers both
the all-durable case and a missing event. A production integration test should
run the same comparison while injecting database rollback/crash boundaries.
