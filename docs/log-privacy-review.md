# Log privacy review

## Procedure

1. Export a representative structured-log sample from ingestion, projection,
   observer, report, and retention workers.
2. Run `scan_log_line` over each serialized line.
3. Fail the review if `Address`, `Signature`, or `PrivacyField` is returned.
4. Preserve only the category and source component in the incident record;
   never copy the offending value into the report.

The scanner targets field names (`wallet_address`, `signature`,
`privacy_mode`, `fingerprint`, and credential-bearing variants). It is a
review aid, not a substitute for log-schema allowlists and runtime redaction.

## Test

Run `cargo test -p landfall-server log_safety`; the corpus covers address,
signature, privacy-field, and clean lines.
