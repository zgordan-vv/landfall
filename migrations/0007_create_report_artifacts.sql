-- Report metadata and immutable exported artifacts.
CREATE TABLE IF NOT EXISTS reporting.report_metadata (
    report_id uuid PRIMARY KEY REFERENCES reporting.report_runs(report_id),
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    environment_id uuid REFERENCES control.environments(environment_id),
    title text NOT NULL CHECK (length(title) BETWEEN 1 AND 240),
    semantics_version text NOT NULL,
    trace_count integer NOT NULL CHECK (trace_count >= 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reporting.report_artifacts (
    artifact_id uuid PRIMARY KEY,
    report_id uuid NOT NULL REFERENCES reporting.report_runs(report_id),
    format text NOT NULL CHECK (format IN ('json', 'html')),
    privacy_profile text NOT NULL CHECK (privacy_profile IN ('internal', 'shareable')),
    storage_key text NOT NULL CHECK (length(storage_key) BETWEEN 1 AND 1024),
    content_sha256 bytea NOT NULL CHECK (octet_length(content_sha256) = 32),
    content_bytes bigint NOT NULL CHECK (content_bytes >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (report_id, format, privacy_profile)
);

CREATE INDEX IF NOT EXISTS report_artifacts_report_idx
    ON reporting.report_artifacts (report_id, created_at);
