# Ingestion HTTP semantics

The response is based on the committed ingestion outcome, not merely request
parsing. If at least one event was newly inserted, the API returns `202
Accepted`; a batch containing only previously claimed event IDs returns `200
OK` and reports its duplicate count. Malformed, incompatible, privacy-unsafe,
or oversized input uses a structured `4xx` error and is never partially
accepted.
