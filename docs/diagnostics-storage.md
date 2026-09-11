# Diagnostics and recommendation storage

Migration `0005_create_diagnostics_recommendations.sql` persists derived
findings separately from immutable telemetry. Diagnostics retain rule and
certainty versions plus many-to-many event evidence. Recommendations link back
to diagnostics and keep an append-only disposition history (`accepted`,
`rejected`, `implemented`, or `not_applicable`) for auditability.
