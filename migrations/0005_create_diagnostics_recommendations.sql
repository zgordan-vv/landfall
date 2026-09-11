-- Evidence-linked derived findings and append-only recommendation history.
CREATE TABLE IF NOT EXISTS reporting.diagnostics (
    diagnostic_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    rule_id text NOT NULL,
    rule_set_version text NOT NULL,
    claim_key text NOT NULL,
    certainty text NOT NULL CHECK (certainty IN ('confirmed', 'probable', 'unknown')),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reporting.diagnostic_evidence (
    diagnostic_id uuid NOT NULL REFERENCES reporting.diagnostics(diagnostic_id),
    event_id uuid NOT NULL,
    PRIMARY KEY (diagnostic_id, event_id)
);

CREATE TABLE IF NOT EXISTS reporting.recommendations (
    recommendation_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    recommendation_key text NOT NULL,
    rule_set_version text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reporting.recommendation_diagnostics (
    recommendation_id uuid NOT NULL REFERENCES reporting.recommendations(recommendation_id),
    diagnostic_id uuid NOT NULL REFERENCES reporting.diagnostics(diagnostic_id),
    PRIMARY KEY (recommendation_id, diagnostic_id)
);

CREATE TABLE IF NOT EXISTS reporting.recommendation_evidence (
    recommendation_id uuid NOT NULL REFERENCES reporting.recommendations(recommendation_id),
    event_id uuid NOT NULL,
    PRIMARY KEY (recommendation_id, event_id)
);

CREATE TABLE IF NOT EXISTS reporting.recommendation_dispositions (
    disposition_id uuid PRIMARY KEY,
    recommendation_id uuid NOT NULL REFERENCES reporting.recommendations(recommendation_id),
    disposition text NOT NULL CHECK (disposition IN ('accepted', 'rejected', 'implemented', 'not_applicable')),
    reason text,
    recorded_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS diagnostics_trace_idx ON reporting.diagnostics (trace_id, created_at);
CREATE INDEX IF NOT EXISTS recommendations_trace_idx ON reporting.recommendations (trace_id, created_at);
CREATE INDEX IF NOT EXISTS dispositions_recommendation_idx ON reporting.recommendation_dispositions (recommendation_id, recorded_at);
