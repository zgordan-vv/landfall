-- Per-trace projection progress, separate from business state.
CREATE TABLE IF NOT EXISTS work.projection_watermarks (
    trace_id uuid PRIMARY KEY REFERENCES reporting.traces(trace_id),
    projection_version bigint NOT NULL CHECK (projection_version >= 0),
    updated_at timestamptz NOT NULL DEFAULT now()
);
