-- Trace graph and typed child projections.
CREATE TABLE IF NOT EXISTS reporting.business_actions (
    business_action_id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    environment_id uuid NOT NULL REFERENCES control.environments(environment_id),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reporting.traces (
    trace_id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    environment_id uuid NOT NULL REFERENCES control.environments(environment_id),
    business_action_id uuid REFERENCES reporting.business_actions(business_action_id),
    lifecycle_state text NOT NULL DEFAULT 'created',
    landing_state text NOT NULL DEFAULT 'not_observed',
    execution_state text NOT NULL DEFAULT 'unknown',
    application_state text NOT NULL DEFAULT 'unknown',
    observation_state text NOT NULL DEFAULT 'in_progress',
    updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reporting.trace_aliases (
    left_trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    right_trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    relationship text NOT NULL CHECK (relationship IN ('same_signed_transaction_alias_candidate', 'replacement')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (left_trace_id, right_trace_id),
    CHECK (left_trace_id <> right_trace_id)
);

CREATE TABLE IF NOT EXISTS reporting.submission_attempts (
    attempt_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    route_id uuid REFERENCES control.routes(route_id),
    started_event_id uuid,
    completed_event_id uuid,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reporting.simulations (
    simulation_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    started_event_id uuid,
    completed_event_id uuid
);

CREATE TABLE IF NOT EXISTS reporting.status_observations (
    observation_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    source_id uuid,
    observed_at timestamptz NOT NULL
);

CREATE TABLE IF NOT EXISTS reporting.execution_metadata (
    execution_metadata_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    slot bigint,
    execution_result text NOT NULL CHECK (execution_result IN ('success', 'failure')),
    captured_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS traces_environment_updated_idx ON reporting.traces (environment_id, updated_at, trace_id);
CREATE INDEX IF NOT EXISTS attempts_trace_idx ON reporting.submission_attempts (trace_id, created_at);
