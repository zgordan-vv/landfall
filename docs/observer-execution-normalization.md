# Execution normalization

`normalize_execution()` maps `getTransaction` JSON into bounded neutral
evidence: exact decimal `slot`, fee and compute-unit values, optional block
time, legacy/v0/unsupported transaction version, log presence, and an
execution-error flag. Raw log contents and provider-specific error objects are
not copied into the normalized model.
