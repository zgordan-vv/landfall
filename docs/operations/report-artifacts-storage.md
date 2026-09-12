# Report artifacts storage

Report metadata stays in PostgreSQL while exported JSON/HTML content is
represented by an immutable artifact manifest. The manifest records format,
privacy profile, storage key, SHA-256 digest, and byte size, allowing object
storage to be replaced or revalidated without changing the report projection.
