# Signature lookup safeguards

`lookup_signature()` denies lookup in `strict` privacy mode and rejects empty
signatures. Standard and full modes return only the signature, normalized
status, and a boolean `logs_present`; log contents and other raw transaction
details are never included in the lookup response.
