# Reducer adapter

`reduce_loaded_events` is the boundary between PostgreSQL rows and the pure
core engine. It deserializes each payload into `WireEvent`, converts receipt
timestamps, applies canonical ordering, and invokes `reduce_trace`. Parsing,
ordering, or invariant failures are returned without producing a partial
projection.
