# Authenticated system status

`authenticated_status()` requires the `system:read` bearer-token scope before
returning detailed operational state. The payload covers database readiness,
projection queue depth and lag, observer route health, retention, and the
active schema/rule versions. Credentials, connection strings, and raw provider
details are not included. Overall status is `degraded` when the database is
unready or any observer route is unhealthy.
