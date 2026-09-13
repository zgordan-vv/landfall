//! Non-custodial spend-policy evaluation for x402 payment requirements.

use std::collections::BTreeSet;

/// A policy that must be satisfied before an agent may ask its wallet to sign.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SpendPolicy {
    /// Stable owner-chosen identity of the agent.
    pub agent_id: String,
    /// CAIP-2 network identifier declared by the x402 requirement.
    pub network: String,
    /// Asset identifier declared by the x402 requirement.
    pub asset: String,
    /// Largest permitted single payment in atomic units.
    pub max_per_request_atomic: u128,
    /// Largest permitted aggregate daily payment in atomic units.
    pub max_per_day_atomic: u128,
    /// Exact normalized merchant origins allowed by this policy.
    pub merchant_origins: BTreeSet<String>,
    /// Disabled policies deny every request.
    pub enabled: bool,
}

/// A normalized x402 payment requirement, before any wallet is invoked.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PaymentRequest<'a> {
    /// Agent attempting to buy the resource.
    pub agent_id: &'a str,
    /// Normalized resource origin, not an arbitrary full URL.
    pub merchant_origin: &'a str,
    /// CAIP-2 network requested by the merchant.
    pub network: &'a str,
    /// Asset requested by the merchant.
    pub asset: &'a str,
    /// Canonical base-10 atomic-unit amount.
    pub amount_atomic: &'a str,
    /// Already approved/settled amount for this policy in the current UTC day.
    pub spent_today_atomic: u128,
}

/// A deterministic reason an x402 request may or may not proceed to a wallet.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum PolicyDecision {
    /// All policy conditions passed. This is not a payment or a settlement.
    Approved,
    /// The policy is disabled.
    PolicyDisabled,
    /// The request came from another agent identity.
    AgentMismatch,
    /// The requested chain differs from policy.
    NetworkMismatch,
    /// The requested token differs from policy.
    AssetMismatch,
    /// The merchant origin is not explicitly allowed.
    MerchantNotAllowed,
    /// Amount is not a canonical positive decimal integer.
    InvalidAmount,
    /// The request alone exceeds the per-request maximum.
    PerRequestLimitExceeded,
    /// The request would exceed the current UTC-day spend cap.
    DailyLimitExceeded,
}

/// Evaluates a requirement without signing, transferring, or settling funds.
#[must_use]
pub fn evaluate(policy: &SpendPolicy, request: &PaymentRequest<'_>) -> PolicyDecision {
    if !policy.enabled {
        return PolicyDecision::PolicyDisabled;
    }
    if policy.agent_id != request.agent_id {
        return PolicyDecision::AgentMismatch;
    }
    if policy.network != request.network {
        return PolicyDecision::NetworkMismatch;
    }
    if policy.asset != request.asset {
        return PolicyDecision::AssetMismatch;
    }
    if !policy.merchant_origins.contains(request.merchant_origin) {
        return PolicyDecision::MerchantNotAllowed;
    }
    let Some(amount) = parse_atomic_amount(request.amount_atomic) else {
        return PolicyDecision::InvalidAmount;
    };
    if amount > policy.max_per_request_atomic {
        return PolicyDecision::PerRequestLimitExceeded;
    }
    if request.spent_today_atomic.saturating_add(amount) > policy.max_per_day_atomic {
        return PolicyDecision::DailyLimitExceeded;
    }
    PolicyDecision::Approved
}

fn parse_atomic_amount(value: &str) -> Option<u128> {
    if value.is_empty() || value == "0" || (value.len() > 1 && value.starts_with('0')) {
        return None;
    }
    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{PaymentRequest, PolicyDecision, SpendPolicy, evaluate};
    use std::collections::BTreeSet;

    fn policy() -> SpendPolicy {
        SpendPolicy {
            agent_id: "research-agent".into(),
            network: "solana:mainnet".into(),
            asset: "USDC".into(),
            max_per_request_atomic: 1_000_000,
            max_per_day_atomic: 3_000_000,
            merchant_origins: BTreeSet::from(["https://api.example.com".into()]),
            enabled: true,
        }
    }

    fn request() -> PaymentRequest<'static> {
        PaymentRequest {
            agent_id: "research-agent",
            merchant_origin: "https://api.example.com",
            network: "solana:mainnet",
            asset: "USDC",
            amount_atomic: "1000000",
            spent_today_atomic: 1_000_000,
        }
    }

    #[test]
    fn approves_a_policy_bounded_allowed_request() {
        assert_eq!(evaluate(&policy(), &request()), PolicyDecision::Approved);
    }

    #[test]
    fn never_approves_unbounded_or_noncanonical_amounts() {
        for amount in [
            "",
            "0",
            "01",
            "1.5",
            "-1",
            "340282366920938463463374607431768211456",
        ] {
            let mut candidate = request();
            candidate.amount_atomic = amount;
            assert_eq!(
                evaluate(&policy(), &candidate),
                PolicyDecision::InvalidAmount
            );
        }
    }

    #[test]
    fn enforces_merchant_and_daily_boundaries_before_wallet_use() {
        let mut candidate = request();
        candidate.merchant_origin = "https://untrusted.example";
        assert_eq!(
            evaluate(&policy(), &candidate),
            PolicyDecision::MerchantNotAllowed
        );

        candidate = request();
        candidate.spent_today_atomic = 2_500_000;
        assert_eq!(
            evaluate(&policy(), &candidate),
            PolicyDecision::DailyLimitExceeded
        );
    }
}
