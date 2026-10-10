#!/usr/bin/env bash
set -euo pipefail

service="${LANDFALL_DB_SERVICE:-postgres}"
db_user="${LANDFALL_POSTGRES_USER:-landfall}"
db_name="${LANDFALL_POSTGRES_DB:-landfall}"
project_id="${LANDFALL_PUBLIC_DEMO_PROJECT_ID:-0198ef00-0000-7000-8000-000000000100}"
environment_id="0198ef00-0000-7000-8000-000000000200"
candidate_environment_id="0198ef00-0000-7000-8000-000000000201"
policy_id="0198ef00-0000-7000-8000-000000000900"
report_id="0198ef00-0000-7000-8000-000000000a00"
json_artifact_id="0198ef00-0000-7000-8000-000000000a01"
html_artifact_id="0198ef00-0000-7000-8000-000000000a02"

bash scripts/compose.sh exec -T "$service" psql --username="$db_user" --dbname="$db_name" \
  -v ON_ERROR_STOP=1 \
  -v project_id="$project_id" \
  -v environment_id="$environment_id" \
  -v candidate_environment_id="$candidate_environment_id" \
  -v policy_id="$policy_id" \
  -v report_id="$report_id" \
  -v json_artifact_id="$json_artifact_id" \
  -v html_artifact_id="$html_artifact_id" <<'SQL'
INSERT INTO control.projects (project_id, name)
VALUES (:'project_id'::uuid, 'local-demo')
ON CONFLICT (project_id) DO UPDATE SET name = EXCLUDED.name;
INSERT INTO control.environments (environment_id, project_id, name, cluster)
VALUES
    (:'environment_id'::uuid, :'project_id'::uuid, 'devnet-primary', 'devnet'),
    (:'candidate_environment_id'::uuid, :'project_id'::uuid, 'devnet-fallback', 'devnet')
ON CONFLICT (environment_id) DO UPDATE SET name = EXCLUDED.name, cluster = EXCLUDED.cluster;

INSERT INTO reporting.business_actions (business_action_id, project_id, environment_id, created_at)
VALUES
    ('0198ef00-0000-7000-8000-000000000301'::uuid, :'project_id'::uuid, :'environment_id'::uuid, now() - interval '70 minutes'),
    ('0198ef00-0000-7000-8000-000000000302'::uuid, :'project_id'::uuid, :'environment_id'::uuid, now() - interval '55 minutes'),
    ('0198ef00-0000-7000-8000-000000000303'::uuid, :'project_id'::uuid, :'environment_id'::uuid, now() - interval '45 minutes'),
    ('0198ef00-0000-7000-8000-000000000304'::uuid, :'project_id'::uuid, :'candidate_environment_id'::uuid, now() - interval '35 minutes'),
    ('0198ef00-0000-7000-8000-000000000305'::uuid, :'project_id'::uuid, :'candidate_environment_id'::uuid, now() - interval '22 minutes'),
    ('0198ef00-0000-7000-8000-000000000306'::uuid, :'project_id'::uuid, :'candidate_environment_id'::uuid, now() - interval '10 minutes')
ON CONFLICT (business_action_id) DO NOTHING;

INSERT INTO reporting.traces (
    trace_id, project_id, environment_id, business_action_id, lifecycle_state,
    landing_state, execution_state, application_state, observation_state, updated_at
)
VALUES
    ('0198ef00-0000-7000-8000-000000000401'::uuid, :'project_id'::uuid, :'environment_id'::uuid, '0198ef00-0000-7000-8000-000000000301'::uuid, 'observed', 'finalized', 'success', 'success', 'observed', now() - interval '70 minutes'),
    ('0198ef00-0000-7000-8000-000000000402'::uuid, :'project_id'::uuid, :'environment_id'::uuid, '0198ef00-0000-7000-8000-000000000302'::uuid, 'observed', 'confirmed', 'failure', 'failure', 'observed', now() - interval '55 minutes'),
    ('0198ef00-0000-7000-8000-000000000403'::uuid, :'project_id'::uuid, :'environment_id'::uuid, '0198ef00-0000-7000-8000-000000000303'::uuid, 'submitted', 'not_observed', 'unknown', 'unknown', 'in_progress', now() - interval '45 minutes'),
    ('0198ef00-0000-7000-8000-000000000404'::uuid, :'project_id'::uuid, :'candidate_environment_id'::uuid, '0198ef00-0000-7000-8000-000000000304'::uuid, 'observed', 'finalized', 'success', 'success', 'observed', now() - interval '35 minutes'),
    ('0198ef00-0000-7000-8000-000000000405'::uuid, :'project_id'::uuid, :'candidate_environment_id'::uuid, '0198ef00-0000-7000-8000-000000000305'::uuid, 'observed', 'finalized', 'success', 'success', 'observed', now() - interval '22 minutes'),
    ('0198ef00-0000-7000-8000-000000000406'::uuid, :'project_id'::uuid, :'candidate_environment_id'::uuid, '0198ef00-0000-7000-8000-000000000306'::uuid, 'signed', 'not_observed', 'unknown', 'unknown', 'in_progress', now() - interval '10 minutes')
ON CONFLICT (trace_id) DO UPDATE SET
    environment_id = EXCLUDED.environment_id,
    lifecycle_state = EXCLUDED.lifecycle_state,
    landing_state = EXCLUDED.landing_state,
    execution_state = EXCLUDED.execution_state,
    application_state = EXCLUDED.application_state,
    observation_state = EXCLUDED.observation_state,
    updated_at = EXCLUDED.updated_at;

INSERT INTO control.x402_spend_policies (
    policy_id, project_id, agent_id, network, asset,
    max_per_request_atomic, max_per_day_atomic, enabled
)
VALUES (
    :'policy_id'::uuid, :'project_id'::uuid, 'landfall-demo-agent',
    'solana-devnet', 'USDC', 2500000, 10000000, true
)
ON CONFLICT (policy_id) DO UPDATE SET
    max_per_request_atomic = EXCLUDED.max_per_request_atomic,
    max_per_day_atomic = EXCLUDED.max_per_day_atomic,
    enabled = EXCLUDED.enabled,
    updated_at = now();

INSERT INTO control.x402_merchant_allowlist (policy_id, merchant_origin)
VALUES
    (:'policy_id'::uuid, 'https://api.demo-merchant.invalid'),
    (:'policy_id'::uuid, 'https://reports.demo-merchant.invalid')
ON CONFLICT (policy_id, merchant_origin) DO NOTHING;

INSERT INTO telemetry.x402_payment_audit (
    audit_id, project_id, policy_id, agent_id, merchant_origin, network, asset,
    amount_atomic, idempotency_key_hash, decision, reason_code, settlement_reference, decided_at
)
VALUES
    ('0198ef00-0000-7000-8000-000000000901'::uuid, :'project_id'::uuid, :'policy_id'::uuid, 'landfall-demo-agent', 'https://api.demo-merchant.invalid', 'solana-devnet', 'USDC', 1200000, decode('0000000000000000000000000000000000000000000000000000000000000901', 'hex'), 'settled', 'policy_approved', 'demo-settlement-901', now() - interval '30 minutes'),
    ('0198ef00-0000-7000-8000-000000000902'::uuid, :'project_id'::uuid, :'policy_id'::uuid, 'landfall-demo-agent', 'https://reports.demo-merchant.invalid', 'solana-devnet', 'USDC', 500000, decode('0000000000000000000000000000000000000000000000000000000000000902', 'hex'), 'approved', 'policy_approved', NULL, now() - interval '12 minutes'),
    ('0198ef00-0000-7000-8000-000000000903'::uuid, :'project_id'::uuid, :'policy_id'::uuid, 'landfall-demo-agent', 'https://api.demo-merchant.invalid', 'solana-devnet', 'USDC', 9000000, decode('0000000000000000000000000000000000000000000000000000000000000903', 'hex'), 'denied', 'amount_exceeds_policy', NULL, now() - interval '5 minutes')
ON CONFLICT (audit_id) DO UPDATE SET
    project_id = EXCLUDED.project_id,
    policy_id = EXCLUDED.policy_id,
    agent_id = EXCLUDED.agent_id,
    merchant_origin = EXCLUDED.merchant_origin,
    network = EXCLUDED.network,
    asset = EXCLUDED.asset,
    amount_atomic = EXCLUDED.amount_atomic,
    idempotency_key_hash = EXCLUDED.idempotency_key_hash,
    decision = EXCLUDED.decision,
    reason_code = EXCLUDED.reason_code,
    settlement_reference = EXCLUDED.settlement_reference,
    decided_at = EXCLUDED.decided_at;

INSERT INTO reporting.report_runs (report_id, created_at, status)
VALUES (:'report_id'::uuid, now() - interval '4 minutes', 'completed')
ON CONFLICT (report_id) DO UPDATE SET status = EXCLUDED.status;

INSERT INTO reporting.report_metadata (
    report_id, project_id, environment_id, title, semantics_version, trace_count, created_at
)
VALUES (
    :'report_id'::uuid, :'project_id'::uuid, NULL,
    'Demo lifecycle evidence export', 'demo-v1', 6, now() - interval '4 minutes'
)
ON CONFLICT (report_id) DO UPDATE SET
    project_id = EXCLUDED.project_id,
    environment_id = EXCLUDED.environment_id,
    title = EXCLUDED.title,
    semantics_version = EXCLUDED.semantics_version,
    trace_count = EXCLUDED.trace_count,
    created_at = EXCLUDED.created_at;

WITH report_json AS (
    SELECT convert_to('{"title":"Demo lifecycle evidence export","privacy_profile":"shareable","traces":6}'::text, 'UTF8') AS content
),
report_html AS (
    SELECT convert_to('<!doctype html><title>Demo lifecycle evidence export</title><h1>Demo lifecycle evidence export</h1><p>Six demo traces across two devnet environments.</p>'::text, 'UTF8') AS content
)
INSERT INTO reporting.report_artifacts (
    artifact_id, report_id, format, privacy_profile, storage_key,
    content_sha256, content_bytes, content, created_at
)
SELECT :'json_artifact_id'::uuid, :'report_id'::uuid, 'json', 'shareable',
       'reports/demo-lifecycle-evidence.json',
       decode('0000000000000000000000000000000000000000000000000000000000000a01', 'hex'),
       octet_length(content), content, now() - interval '4 minutes'
FROM report_json
ON CONFLICT (report_id, format, privacy_profile) DO UPDATE SET
    content_bytes = EXCLUDED.content_bytes,
    content = EXCLUDED.content,
    created_at = EXCLUDED.created_at;

WITH report_html AS (
    SELECT convert_to('<!doctype html><title>Demo lifecycle evidence export</title><h1>Demo lifecycle evidence export</h1><p>Six demo traces across two devnet environments.</p>'::text, 'UTF8') AS content
)
INSERT INTO reporting.report_artifacts (
    artifact_id, report_id, format, privacy_profile, storage_key,
    content_sha256, content_bytes, content, created_at
)
SELECT :'html_artifact_id'::uuid, :'report_id'::uuid, 'html', 'shareable',
       'reports/demo-lifecycle-evidence.html',
       decode('0000000000000000000000000000000000000000000000000000000000000a02', 'hex'),
       octet_length(content), content, now() - interval '4 minutes'
FROM report_html
ON CONFLICT (report_id, format, privacy_profile) DO UPDATE SET
    content_bytes = EXCLUDED.content_bytes,
    content = EXCLUDED.content,
    created_at = EXCLUDED.created_at;
SQL
printf 'Seeded project_id=%s environment_id=%s\n' "$project_id" "$environment_id"
