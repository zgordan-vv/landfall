# SDK UUIDv7 identities

The SDK generates lowercase canonical UUIDv7 values: the first 48 bits encode
milliseconds since Unix epoch, while the version and variant bits are fixed by
the format. `isCanonicalUuidV7` validates incoming IDs before they become event
or trace identities. These IDs are identifiers only, never authentication
secrets.
