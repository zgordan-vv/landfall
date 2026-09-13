//! Durable x402 spend-policy repository operations.

use landfall_core::x402::{PaymentRequest, PolicyDecision, SpendPolicy, evaluate};
use sqlx::{PgPool, Row};
use std::collections::BTreeSet;
use uuid::Uuid;

/// Durable policy representation without wallet or signing material.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct X402SpendPolicyRecord {
    /// Stable policy identifier.
    pub policy_id: Uuid,
    /// Owning project scope.
    pub project_id: Uuid,
    /// Agent identity controlled by the project.
    pub agent_id: String,
    /// CAIP-2 network.
    pub network: String,
    /// Asset identifier.
    pub asset: String,
    /// Maximum atomic amount for one request.
    pub max_per_request_atomic: String,
    /// Maximum atomic amount over a UTC day.
    pub max_per_day_atomic: String,
    /// Whether evaluation may approve this policy.
    pub enabled: bool,
    /// Exact normalized HTTPS merchant origins.
    pub merchant_origins: Vec<String>,
}

/// Idempotent durable result of a pre-payment authorization.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct X402AuthorizationRecord {
    /// Immutable audit record identifier.
    pub audit_id: Uuid,
    /// Decision made before any wallet call.
    pub decision: PolicyDecision,
    /// Stable machine-readable decision reason.
    pub reason_code: String,
    /// Whether the result was a prior idempotent decision.
    pub replayed: bool,
}

/// Durable result of recording a non-custodial payment outcome.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct X402SettlementRecord {
    /// The pre-payment audit record whose state changed.
    pub audit_id: Uuid,
    /// Final terminal state, either `settled` or `failed`.
    pub outcome: X402SettlementOutcome,
    /// Whether this was an idempotent repeat of a terminal outcome.
    pub replayed: bool,
}

/// A privacy-safe entry from the immutable x402 payment ledger.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct X402PaymentAuditRecord {
    /// Durable payment-decision identifier.
    pub audit_id: Uuid,
    /// Policy that governed the request, if one was found.
    pub policy_id: Option<Uuid>,
    /// Project-controlled agent that requested payment.
    pub agent_id: String,
    /// Allowed merchant origin, not the full resource URL.
    pub merchant_origin: String,
    /// x402 network identifier.
    pub network: String,
    /// Asset identifier.
    pub asset: String,
    /// Requested amount in atomic units.
    pub amount_atomic: String,
    /// `approved`, `denied`, `settled`, or `failed`.
    pub decision: String,
    /// Stable machine-readable explanation.
    pub reason_code: String,
    /// Opaque external receipt, if provided; never a signed payment payload.
    pub settlement_reference: Option<String>,
    /// Server-side decision time in UTC.
    pub decided_at: String,
}

/// The only terminal outcomes an external wallet/facilitator may report.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum X402SettlementOutcome {
    /// The merchant/facilitator completed the approved payment.
    Settled,
    /// Signing, delivery, or facilitator settlement failed.
    Failed,
}

impl X402SettlementOutcome {
    /// Stable API/database spelling for the terminal state.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Settled => "settled",
            Self::Failed => "failed",
        }
    }

    fn from_db(value: &str) -> Option<Self> {
        match value {
            "settled" => Some(Self::Settled),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// Evaluates and durably records an idempotent x402 pre-payment decision.
pub async fn authorize_x402_payment(
    pool: &PgPool,
    project_id: Uuid,
    policy_id: Uuid,
    agent_id: &str,
    merchant_origin: &str,
    network: &str,
    asset: &str,
    amount_atomic: &str,
    idempotency_key_hash: &[u8],
) -> Result<X402AuthorizationRecord, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if let Some(row) = sqlx::query("SELECT audit_id, decision, reason_code FROM telemetry.x402_payment_audit WHERE project_id = $1 AND idempotency_key_hash = $2")
        .bind(project_id).bind(idempotency_key_hash).fetch_optional(&mut *tx).await? {
        tx.commit().await?;
        return Ok(X402AuthorizationRecord { audit_id: row.try_get("audit_id")?, decision: decision_from_db(&row.try_get::<String, _>("decision")?), reason_code: row.try_get("reason_code")?, replayed: true });
    }
    let row = sqlx::query("SELECT agent_id, network, asset, max_per_request_atomic::text AS per_request, max_per_day_atomic::text AS per_day, enabled FROM control.x402_spend_policies WHERE policy_id = $1 AND project_id = $2 FOR UPDATE")
        .bind(policy_id).bind(project_id).fetch_optional(&mut *tx).await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(X402AuthorizationRecord {
            audit_id: Uuid::now_v7(),
            decision: PolicyDecision::PolicyDisabled,
            reason_code: "policy_not_found".into(),
            replayed: false,
        });
    };
    let origins = sqlx::query_scalar(
        "SELECT merchant_origin FROM control.x402_merchant_allowlist WHERE policy_id = $1",
    )
    .bind(policy_id)
    .fetch_all(&mut *tx)
    .await?;
    let spent: String = sqlx::query_scalar("SELECT COALESCE(SUM(amount_atomic), 0)::text FROM telemetry.x402_payment_audit WHERE policy_id = $1 AND decision IN ('approved', 'settled') AND decided_at >= date_trunc('day', now())")
        .bind(policy_id).fetch_one(&mut *tx).await?;
    let policy = SpendPolicy {
        agent_id: row.try_get("agent_id")?,
        network: row.try_get("network")?,
        asset: row.try_get("asset")?,
        max_per_request_atomic: row
            .try_get::<String, _>("per_request")?
            .parse()
            .unwrap_or(0),
        max_per_day_atomic: row.try_get::<String, _>("per_day")?.parse().unwrap_or(0),
        merchant_origins: origins.into_iter().collect::<BTreeSet<_>>(),
        enabled: row.try_get("enabled")?,
    };
    let decision = evaluate(
        &policy,
        &PaymentRequest {
            agent_id,
            merchant_origin,
            network,
            asset,
            amount_atomic,
            spent_today_atomic: spent.parse().unwrap_or(u128::MAX),
        },
    );
    let reason_code = reason_code(decision).to_owned();
    let audit_id = Uuid::now_v7();
    sqlx::query("INSERT INTO telemetry.x402_payment_audit (audit_id, project_id, policy_id, agent_id, merchant_origin, network, asset, amount_atomic, idempotency_key_hash, decision, reason_code) VALUES ($1, $2, $3, $4, $5, $6, $7, $8::numeric, $9, $10, $11)")
        .bind(audit_id).bind(project_id).bind(policy_id).bind(agent_id).bind(merchant_origin).bind(network).bind(asset).bind(amount_atomic).bind(idempotency_key_hash).bind(decision_to_db(decision)).bind(&reason_code).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(X402AuthorizationRecord {
        audit_id,
        decision,
        reason_code,
        replayed: false,
    })
}

/// Records a terminal outcome after an external signer/facilitator has acted.
///
/// This accepts only an opaque settlement reference. It intentionally has no
/// parameter for a payment signature, wallet key, or serialized transaction.
pub async fn record_x402_settlement(
    pool: &PgPool,
    project_id: Uuid,
    audit_id: Uuid,
    outcome: X402SettlementOutcome,
    reason_code: &str,
    settlement_reference: Option<&str>,
) -> Result<Option<X402SettlementRecord>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT decision FROM telemetry.x402_payment_audit WHERE audit_id = $1 AND project_id = $2 FOR UPDATE",
    )
    .bind(audit_id)
    .bind(project_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let current: String = row.try_get("decision")?;
    if let Some(existing) = X402SettlementOutcome::from_db(&current) {
        tx.commit().await?;
        return Ok(Some(X402SettlementRecord {
            audit_id,
            outcome: existing,
            replayed: true,
        }));
    }
    if current != "approved" {
        tx.commit().await?;
        return Ok(None);
    }
    sqlx::query(
        "UPDATE telemetry.x402_payment_audit SET decision = $3, reason_code = $4, settlement_reference = $5 WHERE audit_id = $1 AND project_id = $2",
    )
    .bind(audit_id)
    .bind(project_id)
    .bind(outcome.as_str())
    .bind(reason_code)
    .bind(settlement_reference)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Some(X402SettlementRecord {
        audit_id,
        outcome,
        replayed: false,
    }))
}

/// Lists the newest safe payment-audit entries for one project.
pub async fn list_x402_payment_audit(
    pool: &PgPool,
    project_id: Uuid,
    limit: i64,
) -> Result<Vec<X402PaymentAuditRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT audit_id, policy_id, agent_id, merchant_origin, network, asset, amount_atomic::text AS amount_atomic, decision, reason_code, settlement_reference, decided_at::text AS decided_at FROM telemetry.x402_payment_audit WHERE project_id = $1 ORDER BY decided_at DESC, audit_id DESC LIMIT $2",
    )
    .bind(project_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(X402PaymentAuditRecord {
                audit_id: row.try_get("audit_id")?,
                policy_id: row.try_get("policy_id")?,
                agent_id: row.try_get("agent_id")?,
                merchant_origin: row.try_get("merchant_origin")?,
                network: row.try_get("network")?,
                asset: row.try_get("asset")?,
                amount_atomic: row.try_get("amount_atomic")?,
                decision: row.try_get("decision")?,
                reason_code: row.try_get("reason_code")?,
                settlement_reference: row.try_get("settlement_reference")?,
                decided_at: row.try_get("decided_at")?,
            })
        })
        .collect()
}

fn decision_to_db(decision: PolicyDecision) -> &'static str {
    if decision == PolicyDecision::Approved {
        "approved"
    } else {
        "denied"
    }
}
fn decision_from_db(value: &str) -> PolicyDecision {
    if value == "approved" || value == "settled" {
        PolicyDecision::Approved
    } else {
        PolicyDecision::PolicyDisabled
    }
}
fn reason_code(decision: PolicyDecision) -> &'static str {
    match decision {
        PolicyDecision::Approved => "approved",
        PolicyDecision::PolicyDisabled => "policy_disabled",
        PolicyDecision::AgentMismatch => "agent_mismatch",
        PolicyDecision::NetworkMismatch => "network_mismatch",
        PolicyDecision::AssetMismatch => "asset_mismatch",
        PolicyDecision::MerchantNotAllowed => "merchant_not_allowed",
        PolicyDecision::InvalidAmount => "invalid_amount",
        PolicyDecision::PerRequestLimitExceeded => "per_request_limit_exceeded",
        PolicyDecision::DailyLimitExceeded => "daily_limit_exceeded",
    }
}

/// Creates or replaces a policy and its complete allowlist atomically.
pub async fn upsert_x402_spend_policy(
    pool: &PgPool,
    policy: &X402SpendPolicyRecord,
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query(
        "INSERT INTO control.x402_spend_policies (policy_id, project_id, agent_id, network, asset, max_per_request_atomic, max_per_day_atomic, enabled) VALUES ($1, $2, $3, $4, $5, $6::numeric, $7::numeric, $8) ON CONFLICT (policy_id) DO UPDATE SET max_per_request_atomic = EXCLUDED.max_per_request_atomic, max_per_day_atomic = EXCLUDED.max_per_day_atomic, enabled = EXCLUDED.enabled, updated_at = now() WHERE control.x402_spend_policies.project_id = EXCLUDED.project_id RETURNING policy_id",
    )
    .bind(policy.policy_id)
    .bind(policy.project_id)
    .bind(&policy.agent_id)
    .bind(&policy.network)
    .bind(&policy.asset)
    .bind(&policy.max_per_request_atomic)
    .bind(&policy.max_per_day_atomic)
    .bind(policy.enabled)
    .fetch_one(&mut *transaction)
    .await?
    .try_get::<Uuid, _>("policy_id")?;
    let stored_policy_id = policy.policy_id;
    sqlx::query("DELETE FROM control.x402_merchant_allowlist WHERE policy_id = $1")
        .bind(stored_policy_id)
        .execute(&mut *transaction)
        .await?;
    for merchant_origin in &policy.merchant_origins {
        sqlx::query(
            "INSERT INTO control.x402_merchant_allowlist (policy_id, merchant_origin) VALUES ($1, $2)",
        )
        .bind(stored_policy_id)
        .bind(merchant_origin)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await
}

/// Lists all policies and allowlist entries visible to one project.
pub async fn list_x402_spend_policies(
    pool: &PgPool,
    project_id: Uuid,
) -> Result<Vec<X402SpendPolicyRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT p.policy_id, p.project_id, p.agent_id, p.network, p.asset, p.max_per_request_atomic::text AS max_per_request_atomic, p.max_per_day_atomic::text AS max_per_day_atomic, p.enabled, COALESCE(array_agg(a.merchant_origin ORDER BY a.merchant_origin) FILTER (WHERE a.merchant_origin IS NOT NULL), '{}') AS merchant_origins FROM control.x402_spend_policies p LEFT JOIN control.x402_merchant_allowlist a ON a.policy_id = p.policy_id WHERE p.project_id = $1 GROUP BY p.policy_id ORDER BY p.created_at, p.policy_id",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            Ok(X402SpendPolicyRecord {
                policy_id: row.try_get("policy_id")?,
                project_id: row.try_get("project_id")?,
                agent_id: row.try_get("agent_id")?,
                network: row.try_get("network")?,
                asset: row.try_get("asset")?,
                max_per_request_atomic: row.try_get("max_per_request_atomic")?,
                max_per_day_atomic: row.try_get("max_per_day_atomic")?,
                enabled: row.try_get("enabled")?,
                merchant_origins: row.try_get("merchant_origins")?,
            })
        })
        .collect()
}
