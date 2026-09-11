-- Durable event identity registry and daily-partitioned raw telemetry parent.
CREATE TABLE IF NOT EXISTS telemetry.event_dedup (
    event_id uuid PRIMARY KEY,
    first_seen_at timestamptz NOT NULL DEFAULT now(),
    batch_id uuid REFERENCES telemetry.ingest_batches(batch_id),
    payload_hash bytea NOT NULL CHECK (octet_length(payload_hash) BETWEEN 16 AND 128)
);

CREATE TABLE IF NOT EXISTS telemetry.raw_events (
    event_id uuid NOT NULL,
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    environment_id uuid NOT NULL REFERENCES control.environments(environment_id),
    trace_id uuid,
    event_type text NOT NULL CHECK (length(event_type) BETWEEN 1 AND 160),
    occurred_at timestamptz NOT NULL,
    received_at timestamptz NOT NULL DEFAULT now(),
    payload jsonb NOT NULL,
    PRIMARY KEY (event_id, occurred_at)
) PARTITION BY RANGE (occurred_at);

CREATE INDEX IF NOT EXISTS raw_events_environment_time_idx
    ON telemetry.raw_events (environment_id, occurred_at, event_id);
CREATE INDEX IF NOT EXISTS raw_events_trace_time_idx
    ON telemetry.raw_events (trace_id, occurred_at, event_id)
    WHERE trace_id IS NOT NULL;
