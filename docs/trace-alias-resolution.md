# Trace alias resolution

Alias candidates are indexed by `(project_id, environment_id, key_id,
fingerprint)`. A repeated fingerprint yields the first observed trace as the
canonical candidate; a different environment never aliases. The index stores
only versioned fingerprints/digests, not raw signed transaction bytes.
