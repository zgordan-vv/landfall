# Observation evidence history

`ObservationEvidenceLog` models the durable records an observer writes while
polling: the first `null` status, status changes, periodic checkpoints, RPC
error transitions, and terminal evidence. The log is append-only and exposes
records for a storage repository to persist; repeated `null` observations and
unchanged statuses do not create duplicate evidence.
