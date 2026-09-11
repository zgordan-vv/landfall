-- Durable jobs with short lease claims and active deduplication.
CREATE TABLE IF NOT EXISTS work.jobs (
    job_id uuid PRIMARY KEY,
    job_type text NOT NULL REFERENCES work.job_types(job_type),
    dedupe_key text NOT NULL CHECK (length(dedupe_key) BETWEEN 1 AND 512),
    status text NOT NULL DEFAULT 'ready' CHECK (status IN ('ready', 'running', 'completed', 'failed', 'dead_letter')),
    payload jsonb NOT NULL DEFAULT '{}'::jsonb,
    available_at timestamptz NOT NULL DEFAULT now(),
    locked_by text,
    locked_until timestamptz,
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    last_error text,
    created_at timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz
);

CREATE UNIQUE INDEX IF NOT EXISTS jobs_active_dedupe_idx
    ON work.jobs (job_type, dedupe_key)
    WHERE status IN ('ready', 'running');
CREATE INDEX IF NOT EXISTS jobs_claim_idx
    ON work.jobs (available_at, locked_until, job_id)
    WHERE status = 'ready';
