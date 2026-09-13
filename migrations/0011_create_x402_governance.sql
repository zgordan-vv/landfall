-- x402 spend-governance is intentionally non-custodial: no private keys,
-- signed payment payloads, or facilitator credentials are stored here.
CREATE TABLE IF NOT EXISTS control.x402_spend_policies (
    policy_id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    agent_id text NOT NULL CHECK (length(agent_id) BETWEEN 1 AND 160),
    network text NOT NULL CHECK (length(network) BETWEEN 1 AND 160),
    asset text NOT NULL CHECK (length(asset) BETWEEN 1 AND 256),
    max_per_request_atomic numeric(78, 0) NOT NULL CHECK (max_per_request_atomic > 0),
    max_per_day_atomic numeric(78, 0) NOT NULL CHECK (max_per_day_atomic > 0),
    enabled boolean NOT NULL DEFAULT true,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, agent_id, network, asset)
);

CREATE TABLE IF NOT EXISTS control.x402_merchant_allowlist (
    policy_id uuid NOT NULL REFERENCES control.x402_spend_policies(policy_id) ON DELETE CASCADE,
    merchant_origin text NOT NULL CHECK (length(merchant_origin) BETWEEN 1 AND 2048),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (policy_id, merchant_origin)
);

CREATE TABLE IF NOT EXISTS telemetry.x402_payment_audit (
    audit_id uuid PRIMARY KEY,
    project_id uuid NOT NULL REFERENCES control.projects(project_id),
    policy_id uuid REFERENCES control.x402_spend_policies(policy_id),
    agent_id text NOT NULL CHECK (length(agent_id) BETWEEN 1 AND 160),
    merchant_origin text NOT NULL CHECK (length(merchant_origin) BETWEEN 1 AND 2048),
    network text NOT NULL CHECK (length(network) BETWEEN 1 AND 160),
    asset text NOT NULL CHECK (length(asset) BETWEEN 1 AND 256),
    amount_atomic numeric(78, 0) NOT NULL CHECK (amount_atomic > 0),
    idempotency_key_hash bytea NOT NULL CHECK (octet_length(idempotency_key_hash) = 32),
    decision text NOT NULL CHECK (decision IN ('approved', 'denied', 'settled', 'failed')),
    reason_code text NOT NULL CHECK (length(reason_code) BETWEEN 1 AND 80),
    settlement_reference text CHECK (length(settlement_reference) BETWEEN 1 AND 256),
    decided_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (project_id, idempotency_key_hash)
);

CREATE INDEX IF NOT EXISTS x402_spend_policies_project_idx ON control.x402_spend_policies (project_id);
CREATE INDEX IF NOT EXISTS x402_payment_audit_project_decided_idx ON telemetry.x402_payment_audit (project_id, decided_at DESC);
