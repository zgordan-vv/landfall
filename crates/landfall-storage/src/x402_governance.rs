//! Durable x402 spend-policy repository operations.

use sqlx::{PgPool, Row};
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
