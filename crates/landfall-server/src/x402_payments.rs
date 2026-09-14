//! Authenticated pre-payment x402 policy authorization endpoint.

use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use landfall_storage::{X402SettlementOutcome, authorize_x402_payment, record_x402_settlement};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, AppState, AuthenticatedToken};

const MAX_X402_REQUESTS_PER_SECOND_PER_PROJECT: u32 = 20;
const MAX_ACTIVE_X402_RATE_WINDOWS: usize = 10_000;
static X402_RATE_WINDOWS: OnceLock<Mutex<HashMap<Uuid, (Instant, u32)>>> = OnceLock::new();

/// A request to evaluate one x402 payment requirement before wallet signing.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct X402AuthorizeRequest {
    pub policy_id: String,
    pub agent_id: String,
    pub merchant_origin: String,
    pub network: String,
    pub asset: String,
    pub amount_atomic: String,
    /// Caller-chosen stable key; only its SHA-256 is persisted.
    pub idempotency_key: String,
}

/// Durable pre-payment decision. `approved` does not mean paid or settled.
#[derive(Debug, Serialize, ToSchema)]
pub struct X402AuthorizeResponse {
    pub audit_id: String,
    pub decision: String,
    pub reason_code: String,
    pub replayed: bool,
}

/// A terminal result reported by an external non-custodial signer/facilitator.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct X402SettlementRequest {
    pub audit_id: String,
    /// `settled` after the paid resource response, otherwise `failed`.
    pub outcome: String,
    /// Stable operational reason, never a signature or transaction payload.
    pub reason_code: String,
    /// Optional safe facilitator receipt or transaction identifier, never encoded evidence.
    pub settlement_reference: Option<String>,
}

/// Idempotent result of recording the terminal payment state.
#[derive(Debug, Serialize, ToSchema)]
pub struct X402SettlementResponse {
    pub audit_id: String,
    pub outcome: String,
    pub replayed: bool,
}

/// Evaluates an x402 payment requirement and atomically reserves an approved daily budget.
#[utoipa::path(post, path = "/v1/x402/authorize", request_body = X402AuthorizeRequest, responses((status = 200, body = X402AuthorizeResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 503, body = ApiError)))]
pub async fn authorize(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedToken>,
    Json(request): Json<X402AuthorizeRequest>,
) -> axum::response::Response {
    if !x402_rate_allowed(principal.project_id) {
        return rate_limited();
    }
    let Ok(policy_id) = Uuid::parse_str(&request.policy_id) else {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_x402_request",
            "policy_id must be a UUID",
        );
    };
    if !valid_authorization_field(&request.idempotency_key, 256)
        || !valid_authorization_field(&request.agent_id, 160)
        || !valid_https_origin(&request.merchant_origin)
        || !valid_authorization_field(&request.network, 160)
        || !valid_authorization_field(&request.asset, 256)
        || !valid_atomic_amount(&request.amount_atomic)
    {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_x402_request",
            "x402 authorization fields must be non-empty and bounded",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "x402 authorization requires durable storage",
        );
    };
    let key_hash = Sha256::digest(request.idempotency_key.as_bytes());
    match authorize_x402_payment(
        pool,
        principal.project_id,
        policy_id,
        &request.agent_id,
        &request.merchant_origin,
        &request.network,
        &request.asset,
        &request.amount_atomic,
        &key_hash,
    )
    .await
    {
        Ok(record) => (
            StatusCode::OK,
            Json(X402AuthorizeResponse {
                audit_id: record.audit_id.to_string(),
                decision: record
                    .terminal_outcome
                    .map(X402SettlementOutcome::as_str)
                    .unwrap_or_else(|| {
                        if record.decision == landfall_core::x402::PolicyDecision::Approved {
                            "approved"
                        } else {
                            "denied"
                        }
                    })
                    .into(),
                reason_code: record.reason_code,
                replayed: record.replayed,
            }),
        )
            .into_response(),
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "x402 authorization failed",
        ),
    }
}

/// Stores a non-custodial x402 payment result without accepting wallet material.
#[utoipa::path(post, path = "/v1/x402/settlements", request_body = X402SettlementRequest, responses((status = 200, body = X402SettlementResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 503, body = ApiError)))]
pub async fn record_settlement(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedToken>,
    Json(request): Json<X402SettlementRequest>,
) -> axum::response::Response {
    if !x402_rate_allowed(principal.project_id) {
        return rate_limited();
    }
    let Ok(audit_id) = Uuid::parse_str(&request.audit_id) else {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_x402_settlement",
            "audit_id must be a UUID",
        );
    };
    let outcome = match request.outcome.as_str() {
        "settled" => X402SettlementOutcome::Settled,
        "failed" => X402SettlementOutcome::Failed,
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "invalid_x402_settlement",
                "outcome must be settled or failed",
            );
        }
    };
    if !valid_reason_code(&request.reason_code)
        || request
            .settlement_reference
            .as_deref()
            .is_some_and(|value| !valid_reference(value))
    {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_x402_settlement",
            "settlement fields are invalid",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "x402 settlement requires durable storage",
        );
    };
    match record_x402_settlement(
        pool,
        principal.project_id,
        audit_id,
        outcome,
        &request.reason_code,
        request.settlement_reference.as_deref(),
    )
    .await
    {
        Ok(Some(record)) => (
            StatusCode::OK,
            Json(X402SettlementResponse {
                audit_id: record.audit_id.to_string(),
                outcome: record.outcome.as_str().into(),
                replayed: record.replayed,
            }),
        )
            .into_response(),
        Ok(None) => error(
            StatusCode::CONFLICT,
            "invalid_x402_settlement_transition",
            "audit record is missing or not awaiting settlement",
        ),
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "x402 settlement failed",
        ),
    }
}

fn valid_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_reference(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

fn valid_authorization_field(value: &str, max_len: usize) -> bool {
    !value.is_empty() && value.len() <= max_len && !value.chars().any(char::is_control)
}

fn valid_https_origin(value: &str) -> bool {
    let Some(host) = value.strip_prefix("https://") else {
        return false;
    };
    !host.is_empty()
        && host.len() <= 253
        && !host.contains(['/', '?', '#', '@'])
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b':'))
}

fn valid_atomic_amount(value: &str) -> bool {
    value.len() <= 80 && value.bytes().all(|byte| byte.is_ascii_digit()) && !value.starts_with('0')
}

fn error(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> axum::response::Response {
    (status, Json(ApiError { code, message })).into_response()
}

fn rate_limited() -> axum::response::Response {
    error(
        StatusCode::TOO_MANY_REQUESTS,
        "x402_rate_limited",
        "x402 payment-control rate limit exceeded",
    )
}

fn x402_rate_allowed(project_id: Uuid) -> bool {
    let windows = X402_RATE_WINDOWS.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut windows) = windows.lock() else {
        return false;
    };
    let now = Instant::now();
    windows.retain(|_, (started_at, _)| now.duration_since(*started_at) < Duration::from_secs(1));
    let Some(window) = windows.get_mut(&project_id) else {
        if windows.len() >= MAX_ACTIVE_X402_RATE_WINDOWS {
            return false;
        }
        windows.insert(project_id, (now, 1));
        return true;
    };
    if window.1 >= MAX_X402_REQUESTS_PER_SECOND_PER_PROJECT {
        return false;
    }
    window.1 += 1;
    true
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_X402_REQUESTS_PER_SECOND_PER_PROJECT, X402SettlementRequest, valid_atomic_amount,
        valid_https_origin, valid_reference, x402_rate_allowed,
    };
    use uuid::Uuid;

    #[test]
    fn settlement_reference_is_an_identifier_not_a_payload() {
        assert!(valid_reference("5NfVx7Zp9-merchant.receipt:42"));
        assert!(!valid_reference("eyJzaWduYXR1cmUiOiJwYXlsb2FkIn0="));
        assert!(!valid_reference("receipt/with/path"));
    }

    #[test]
    fn authorization_values_are_constrained_before_storage() {
        assert!(valid_https_origin("https://merchant.example:8443"));
        assert!(!valid_https_origin("http://merchant.example"));
        assert!(!valid_https_origin("https://user@merchant.example"));
        assert!(valid_atomic_amount("1000000"));
        assert!(!valid_atomic_amount("0"));
        assert!(!valid_atomic_amount("01"));
        assert!(!valid_atomic_amount("1.5"));
    }

    #[test]
    fn payment_evidence_is_rejected_at_the_json_boundary() {
        let body = r#"{"audit_id":"00000000-0000-0000-0000-000000000000","outcome":"failed","reason_code":"merchant_rejected","payment_signature":"secret"}"#;
        assert!(serde_json::from_str::<X402SettlementRequest>(body).is_err());
    }

    #[test]
    fn x402_rate_limit_is_scoped_to_one_project() {
        let first = Uuid::now_v7();
        let second = Uuid::now_v7();
        for _ in 0..MAX_X402_REQUESTS_PER_SECOND_PER_PROJECT {
            assert!(x402_rate_allowed(first));
        }
        assert!(!x402_rate_allowed(first));
        assert!(x402_rate_allowed(second));
    }
}
