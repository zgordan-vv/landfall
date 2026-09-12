# Offline golden incidents

The CLI can inspect any committed protocol incident fixture without a running
server. For example:

```bash
jq -c '.events[]' fixtures/protocol/v1/valid/incidents/success.batch.json \
  | landfall ingest -
```

The snapshot test converts the fixture's events to NDJSON, runs ingestion,
grouping, analysis, and report export twice, and asserts byte-for-byte stable
JSON semantics. This protects the portfolio demo from accidental nondeterminism
while the report renderer is still evolving.
