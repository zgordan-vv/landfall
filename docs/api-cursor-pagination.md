# Cursor pagination

`landfall-server::pagination::paginate()` accepts an already deterministically
ordered slice, a cursor, and a bounded limit (1–1000). It returns a page and an
opaque `lfc1_…` next cursor. Cursor decoding and limit validation fail closed;
API handlers can map these errors to a stable 400 response.
