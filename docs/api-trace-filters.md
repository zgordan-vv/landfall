# Trace list filters

`validate_trace_filter()` parses optional project, environment, status, and
RFC3339 `from`/`to` filters. A range must be non-negative and no longer than
31 days, preventing expensive unbounded list queries before they reach the
database.
