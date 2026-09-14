-- Persist report bytes with their immutable metadata; report exports survive restarts.
ALTER TABLE reporting.report_artifacts
    ADD COLUMN IF NOT EXISTS content bytea;

ALTER TABLE reporting.report_artifacts
    ADD CONSTRAINT report_artifact_content_size_matches
    CHECK (content IS NULL OR octet_length(content) = content_bytes);
