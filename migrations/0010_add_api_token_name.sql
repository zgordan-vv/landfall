-- Human-readable token labels are control-plane metadata; token plaintext is
-- never stored and the hash remains the only credential material in PostgreSQL.
ALTER TABLE control.api_tokens
    ADD COLUMN IF NOT EXISTS name text;

UPDATE control.api_tokens
SET name = token_prefix
WHERE name IS NULL;

ALTER TABLE control.api_tokens
    ALTER COLUMN name SET NOT NULL;

ALTER TABLE control.api_tokens
    ADD CONSTRAINT api_tokens_name_length
    CHECK (length(name) BETWEEN 1 AND 120);

CREATE UNIQUE INDEX IF NOT EXISTS api_tokens_project_name_idx
    ON control.api_tokens (project_id, name);
