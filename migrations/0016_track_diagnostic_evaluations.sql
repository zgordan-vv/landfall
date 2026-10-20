-- Every re-evaluation of a trace records which diagnostics and recommendations
-- it produced. Rows in reporting.diagnostics and reporting.recommendations stay
-- append-only for audit; readers serve only the membership of the newest
-- evaluation, so findings from partial snapshots are withdrawn once evidence
-- arrives instead of persisting forever.
ALTER TABLE reporting.diagnostics
    ADD COLUMN IF NOT EXISTS unknown_reason text;

CREATE TABLE IF NOT EXISTS reporting.diagnostic_evaluations (
    evaluation_id uuid PRIMARY KEY,
    trace_id uuid NOT NULL REFERENCES reporting.traces(trace_id),
    -- Size of the immutable event snapshot that was evaluated. Raw events are
    -- append-only, so a larger snapshot is never less complete; it wins over a
    -- later-committed evaluation that raced on a smaller snapshot.
    input_event_count bigint NOT NULL CHECK (input_event_count >= 0),
    evaluated_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS reporting.diagnostic_evaluation_diagnostics (
    evaluation_id uuid NOT NULL REFERENCES reporting.diagnostic_evaluations(evaluation_id),
    diagnostic_id uuid NOT NULL REFERENCES reporting.diagnostics(diagnostic_id),
    PRIMARY KEY (evaluation_id, diagnostic_id)
);

CREATE TABLE IF NOT EXISTS reporting.diagnostic_evaluation_recommendations (
    evaluation_id uuid NOT NULL REFERENCES reporting.diagnostic_evaluations(evaluation_id),
    recommendation_id uuid NOT NULL REFERENCES reporting.recommendations(recommendation_id),
    PRIMARY KEY (evaluation_id, recommendation_id)
);

CREATE INDEX IF NOT EXISTS diagnostic_evaluations_latest_idx
    ON reporting.diagnostic_evaluations (trace_id, input_event_count DESC, evaluated_at DESC, evaluation_id DESC);

-- Rows written before evaluations were tracked keep their previous visibility:
-- one legacy evaluation per trace holds them all until the trace is next
-- re-evaluated. input_event_count 0 lets any real evaluation supersede it.
CREATE TEMPORARY TABLE legacy_diagnostic_evaluations ON COMMIT DROP AS
SELECT gen_random_uuid() AS evaluation_id, trace_id, max(created_at) AS evaluated_at
FROM (
    SELECT trace_id, created_at FROM reporting.diagnostics
    UNION ALL
    SELECT trace_id, created_at FROM reporting.recommendations
) AS legacy
GROUP BY trace_id;

INSERT INTO reporting.diagnostic_evaluations (evaluation_id, trace_id, input_event_count, evaluated_at)
SELECT evaluation_id, trace_id, 0, evaluated_at FROM legacy_diagnostic_evaluations;

INSERT INTO reporting.diagnostic_evaluation_diagnostics (evaluation_id, diagnostic_id)
SELECT l.evaluation_id, d.diagnostic_id
FROM legacy_diagnostic_evaluations l
JOIN reporting.diagnostics d ON d.trace_id = l.trace_id;

INSERT INTO reporting.diagnostic_evaluation_recommendations (evaluation_id, recommendation_id)
SELECT l.evaluation_id, r.recommendation_id
FROM legacy_diagnostic_evaluations l
JOIN reporting.recommendations r ON r.trace_id = l.trace_id;
