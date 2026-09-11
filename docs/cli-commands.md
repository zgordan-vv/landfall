# Landfall CLI commands

The CLI now recognizes the operator surface `init`, `doctor`, `trace`,
`report create|status|download`, `rules`, `retention run --dry-run`, and
`demo`. Commands provide concise human output by default; adding `--json`
returns a machine-readable `{ ok, message }` envelope. Existing
`ingest <ndjson-file>` behavior remains unchanged.
