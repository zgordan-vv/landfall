//! Authenticated pre-payment x402 policy authorization endpoint.

use axum::{
    Json,
    extract::{Extension, State},
    http::StatusCode,
    response::IntoResponse,
};
use landfall_storage::authorize_x402_payment;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, AppState, AuthenticatedToken};

/// A request to evaluate one x402 payment requirement before wallet signing.
#[derive(Debug, Deserialize, ToSchema)]
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

/// Evaluates an x402 payment requirement and atomically reserves an approved daily budget.
#[utoipa::path(post, path = "/v1/x402/authorize", request_body = X402AuthorizeRequest, responses((status = 200, body = X402AuthorizeResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 503, body = ApiError)))]
pub async fn authorize(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(principal): Extension<AuthenticatedToken>,
    Json(request): Json<X402AuthorizeRequest>,
) -> axum::response::Response {
    let Ok(policy_id) = Uuid::parse_str(&request.policy_id) else {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_x402_request",
            "policy_id must be a UUID",
        );
    };
    if request.idempotency_key.is_empty()
        || request.idempotency_key.len() > 256
        || request.agent_id.is_empty()
        || request.merchant_origin.is_empty()
        || request.network.is_empty()
        || request.asset.is_empty()
        || request.amount_atomic.is_empty()
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
                decision: if record.decision == landfall_core::x402::PolicyDecision::Approved {
                    "approved".into()
                } else {
                    "denied".into()
                },
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

fn error(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> axum::response::Response {
    (status, Json(ApiError { code, message })).into_response()
}
