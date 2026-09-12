# Projection versions and watermarks

Migration `0008` adds `work.projection_watermarks`, keyed by trace. The
watermark helper advances `projection_version` monotonically and ignores stale
writes, allowing concurrent/replayed jobs without moving the read model
backwards. The table is operational metadata and remains separate from trace
business state.
