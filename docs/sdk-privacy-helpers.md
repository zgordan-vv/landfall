# SDK endpoint and metadata privacy

`redactEndpoint` removes URL userinfo, query, and fragment components before an
endpoint can enter telemetry. `allowMetadata` retains only a small explicit
key set and bounded scalar values, returning a frozen object. Unknown keys and
credential-bearing fields are dropped rather than serialized.
