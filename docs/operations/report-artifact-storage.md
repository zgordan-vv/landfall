# Report artifact storage

Each JSON and HTML export is stored as immutable PostgreSQL bytes alongside its
SHA-256 digest, byte count, privacy profile, and storage key. Artifacts survive
server restarts and are authorized by the owning project's administrator token.

The API returns only the requested format and never exposes an RPC endpoint,
credential, or wallet material.
