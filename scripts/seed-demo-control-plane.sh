#!/usr/bin/env bash
set -euo pipefail

# Seeds only demo configuration: the project, its environments and their
# comparison roles, and an x402 spend policy. Trace, diagnostic, and report data must come from real
# ingestion (scripts/demo-traffic/), never from rows written directly into
# read models. Earlier versions of this script inserted placeholder traces,
# x402 audit rows, and a report; they are removed here by their fixed IDs.

service="${LANDFALL_DB_SERVICE:-postgres}"
db_user="${LANDFALL_POSTGRES_USER:-landfall}"
db_name="${LANDFALL_POSTGRES_DB:-landfall}"
project_id="${LANDFALL_PUBLIC_DEMO_PROJECT_ID:-0198ef00-0000-7000-8000-000000000100}"
environment_id="0198ef00-0000-7000-8000-000000000200"
candidate_environment_id="0198ef00-0000-7000-8000-000000000201"
scenario_environment_id="0198ef00-0000-7000-8000-000000000202"
policy_id="0198ef00-0000-7000-8000-000000000900"

bash scripts/compose.sh exec -T "$service" psql --username="$db_user" --dbname="$db_name" \
  -v ON_ERROR_STOP=1 \
  -v project_id="$project_id" \
  -v environment_id="$environment_id" \
  -v candidate_environment_id="$candidate_environment_id" \
  -v scenario_environment_id="$scenario_environment_id" \
  -v policy_id="$policy_id" <<'SQL'
BEGIN;

INSERT INTO control.projects (project_id, name)
VALUES (:'project_id'::uuid, 'local-demo')
ON CONFLICT (project_id) DO UPDATE SET name = EXCLUDED.name;
-- Descriptions say plainly that the Comparison difference is staged by
-- scripts/demo-traffic/generate.mjs, which sends a failure-heavy scenario mix to
-- the baseline and a mostly healthy mix to the candidate.
INSERT INTO control.environments (environment_id, project_id, name, cluster, description)
VALUES
    (:'environment_id'::uuid, :'project_id'::uuid, 'devnet-baseline', 'devnet',
     'Demo: baseline injects failure scenarios (about 65% of traces are deliberately misconfigured by the traffic generator)'),
    (:'candidate_environment_id'::uuid, :'project_id'::uuid, 'devnet-tuned', 'devnet',
     'Demo: candidate uses tuned settings (about 30% injected failure scenarios); the difference is staged by the traffic generator'),
    (:'scenario_environment_id'::uuid, :'project_id'::uuid, 'scenario-library', 'replayed-fixtures',
     'Replayed golden incident fixtures, not live traffic')
ON CONFLICT (environment_id) DO UPDATE SET
    project_id = EXCLUDED.project_id,
    name = EXCLUDED.name,
    cluster = EXCLUDED.cluster,
    description = EXCLUDED.description;

-- Comparison roles are explicit; they are never inferred from recency.
INSERT INTO control.comparison_cohorts (project_id, baseline_environment_id, candidate_environment_id)
VALUES (:'project_id'::uuid, :'environment_id'::uuid, :'candidate_environment_id'::uuid)
ON CONFLICT (project_id) DO UPDATE SET
    baseline_environment_id = EXCLUDED.baseline_environment_id,
    candidate_environment_id = EXCLUDED.candidate_environment_id,
    updated_at = now();

INSERT INTO control.x402_spend_policies (
    policy_id, project_id, agent_id, network, asset,
    max_per_request_atomic, max_per_day_atomic, enabled
)
VALUES (
    :'policy_id'::uuid, :'project_id'::uuid, 'landfall-demo-agent',
    'solana-devnet', 'USDC', 2500000, 10000000, true
)
ON CONFLICT (policy_id) DO UPDATE SET
    project_id = EXCLUDED.project_id,
    agent_id = EXCLUDED.agent_id,
    network = EXCLUDED.network,
    asset = EXCLUDED.asset,
    max_per_request_atomic = EXCLUDED.max_per_request_atomic,
    max_per_day_atomic = EXCLUDED.max_per_day_atomic,
    enabled = EXCLUDED.enabled,
    updated_at = now();

INSERT INTO control.x402_merchant_allowlist (policy_id, merchant_origin)
VALUES
    (:'policy_id'::uuid, 'https://api.demo-merchant.invalid'),
    (:'policy_id'::uuid, 'https://reports.demo-merchant.invalid')
ON CONFLICT (policy_id, merchant_origin) DO NOTHING;

-- Remove placeholder rows written by earlier versions of this script.
DELETE FROM reporting.report_artifacts
WHERE report_id = '0198ef00-0000-7000-8000-000000000a00'::uuid;
DELETE FROM reporting.report_metadata
WHERE report_id = '0198ef00-0000-7000-8000-000000000a00'::uuid;
DELETE FROM reporting.report_runs
WHERE report_id = '0198ef00-0000-7000-8000-000000000a00'::uuid;
DELETE FROM telemetry.x402_payment_audit
WHERE audit_id IN (
    '0198ef00-0000-7000-8000-000000000901'::uuid,
    '0198ef00-0000-7000-8000-000000000902'::uuid,
    '0198ef00-0000-7000-8000-000000000903'::uuid
);
DELETE FROM reporting.traces
WHERE trace_id IN (
    '0198ef00-0000-7000-8000-000000000401'::uuid, '0198ef00-0000-7000-8000-000000000402'::uuid,
    '0198ef00-0000-7000-8000-000000000403'::uuid, '0198ef00-0000-7000-8000-000000000404'::uuid,
    '0198ef00-0000-7000-8000-000000000405'::uuid, '0198ef00-0000-7000-8000-000000000406'::uuid
)
AND NOT EXISTS (
    SELECT 1 FROM telemetry.raw_events e WHERE e.trace_id = reporting.traces.trace_id
);
DELETE FROM reporting.business_actions
WHERE business_action_id IN (
    '0198ef00-0000-7000-8000-000000000301'::uuid, '0198ef00-0000-7000-8000-000000000302'::uuid,
    '0198ef00-0000-7000-8000-000000000303'::uuid, '0198ef00-0000-7000-8000-000000000304'::uuid,
    '0198ef00-0000-7000-8000-000000000305'::uuid, '0198ef00-0000-7000-8000-000000000306'::uuid
)
AND NOT EXISTS (
    SELECT 1 FROM reporting.traces t WHERE t.business_action_id = reporting.business_actions.business_action_id
);

COMMIT;
SQL
printf 'Seeded project_id=%s\n' "$project_id"
printf '  devnet-baseline   LANDFALL_ENVIRONMENT_ID=%s (comparison baseline)\n' "$environment_id"
printf '  devnet-tuned      LANDFALL_CANDIDATE_ENVIRONMENT_ID=%s (comparison candidate)\n' "$candidate_environment_id"
printf '  scenario-library  LANDFALL_SCENARIO_ENVIRONMENT_ID=%s\n' "$scenario_environment_id"
