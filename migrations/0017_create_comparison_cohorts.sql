-- Explicit comparison roles. The Comparison view must never infer which
-- environment is the baseline from recency, so a project designates both roles
-- here. Environment descriptions say what differs between the cohorts (for
-- example "tuned: compute limit from simulation + headroom").
ALTER TABLE control.environments
    ADD COLUMN IF NOT EXISTS description text
        CHECK (description IS NULL OR length(description) BETWEEN 1 AND 280);

-- Lets the cohort table prove that both environments belong to its project.
ALTER TABLE control.environments
    ADD CONSTRAINT environments_project_environment_key UNIQUE (project_id, environment_id);

CREATE TABLE IF NOT EXISTS control.comparison_cohorts (
    project_id uuid PRIMARY KEY REFERENCES control.projects(project_id),
    baseline_environment_id uuid NOT NULL,
    candidate_environment_id uuid NOT NULL,
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (baseline_environment_id <> candidate_environment_id),
    FOREIGN KEY (project_id, baseline_environment_id)
        REFERENCES control.environments (project_id, environment_id),
    FOREIGN KEY (project_id, candidate_environment_id)
        REFERENCES control.environments (project_id, environment_id)
);
