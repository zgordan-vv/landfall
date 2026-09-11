# Observation schedule and priority queue

`ObservationSchedule` is the in-memory representation of a durable observation
job: stable job/trace IDs, due time, and priority. `ObservationQueue` returns
only due entries, ordered first by due time, then by higher priority, then by
job ID. On restart, ready durable rows are loaded into this queue; the queue
itself is intentionally disposable process state.
