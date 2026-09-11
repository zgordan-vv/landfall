# Reproject command

The CLI selector accepts exactly one replay scope: `--trace ID`,
`--from RFC3339 --until RFC3339`, or `--rule-set-version VERSION`. Parsing is
kept separate from execution so a future database-backed command can reuse the
same validation and cannot accidentally combine scopes.
