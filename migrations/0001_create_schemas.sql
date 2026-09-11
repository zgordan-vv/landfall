-- Landfall persistence foundation. Migrations are append-only after release.
CREATE SCHEMA IF NOT EXISTS control;
CREATE SCHEMA IF NOT EXISTS telemetry;
CREATE SCHEMA IF NOT EXISTS work;
CREATE SCHEMA IF NOT EXISTS reporting;

CREATE TABLE IF NOT EXISTS control.schema_metadata (
    key text PRIMARY KEY,
    value text NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS telemetry.ingest_batches (
    batch_id uuid PRIMARY KEY,
    received_at timestamptz NOT NULL DEFAULT now(),
    event_count integer NOT NULL CHECK (event_count >= 0)
);

CREATE TABLE IF NOT EXISTS work.job_types (
    job_type text PRIMARY KEY,
    description text NOT NULL
);

CREATE TABLE IF NOT EXISTS reporting.report_runs (
    report_id uuid PRIMARY KEY,
    created_at timestamptz NOT NULL DEFAULT now(),
    status text NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed'))
);

INSERT INTO work.job_types (job_type, description) VALUES
    ('project_trace', 'Project canonical trace and derived analysis products')
ON CONFLICT (job_type) DO NOTHING;
