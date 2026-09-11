# Reporting schema

Migration `0004_create_reporting_entities.sql` stores the reconstructed trace
graph separately from raw telemetry. Business actions and traces retain
project/environment scope; aliases require an explicit relationship and cannot
link a trace to itself. Typed child tables keep attempts, simulations, status
observations, and execution metadata queryable without JSON inspection.
