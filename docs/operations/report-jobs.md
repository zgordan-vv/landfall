# Durable report exports

`POST /v1/control/projects/{project_id}/reports` creates a completed report
directly from the project's current durable trace read model. It stores JSON
and HTML artifacts in PostgreSQL in the same transaction as report metadata.

The caller selects `internal` or `shareable` privacy. Shareable reports redact
trace IDs before either artifact is stored. Download JSON or HTML with
`GET /v1/control/projects/{project_id}/reports/{report_id}/{format}`.
