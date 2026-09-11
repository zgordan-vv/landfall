# Server-side privacy enforcement

The ingestion boundary repeats privacy enforcement even when an SDK already
validated the event. Objects and arrays are scanned recursively for prohibited
credential or transport fields (`private_key`, `seed_phrase`, raw transaction
bytes, credentials, headers, and endpoint URLs). The API returns only a stable
`privacy_violation` code and never echoes the rejected value.
