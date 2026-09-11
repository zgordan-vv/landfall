//! Composition root and public application-service surface for Landfall.
#![allow(missing_docs)]

pub mod auth;

use axum::{
    Json, Router,
    extract::State,
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::post,
};
use landfall_protocol::check_event_compatibility;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::info_span;
use uuid::Uuid;

pub const MAX_EVENTS_PER_BATCH: usize = 1_000;
pub const MAX_COMPRESSED_BODY_BYTES: usize = 256 * 1024;
pub const MAX_DECOMPRESSED_BODY_BYTES: usize = 2 * 1024 * 1024;
const PROHIBITED_PRIVACY_KEYS: &[&str] = &[
    "private_key",
    "seed_phrase",
    "signed_transaction_bytes",
    "raw_transaction",
    "authorization",
    "cookie",
    "set_cookie",
    "headers",
    "metadata",
    "endpoint_url",
    "rpc_url",
];

#[derive(Clone, Default)]
pub struct AppState {
    /// Whether dependencies are ready for traffic.
    pub ready: bool,
}

#[derive(Debug, Deserialize)]
pub struct IngestRequest {
    pub events: Vec<Value>,
}

#[derive(Debug, Serialize)]
pub struct IngestAccepted {
    pub accepted: usize,
    pub duplicate: usize,
}

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub code: &'static str,
    pub message: &'static str,
}

fn validate_event(event: &Value) -> Result<(), &'static str> {
    let object = event.as_object().ok_or("event_not_object")?;
    let schema_version = object
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or("missing_schema_version")?;
    let event_type = object
        .get("event_type")
        .and_then(Value::as_str)
        .ok_or("missing_event_type")?;
    check_event_compatibility(schema_version, event_type).map_err(|error| error.code().as_str())
}

fn contains_prohibited_key(value: &Value) -> bool {
    match value {
        Value::Object(object) => object.iter().any(|(key, child)| {
            PROHIBITED_PRIVACY_KEYS.contains(&key.as_str()) || contains_prohibited_key(child)
        }),
        Value::Array(values) => values.iter().any(contains_prohibited_key),
        _ => false,
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/ingest", post(ingest))
        .route("/health/live", axum::routing::get(liveness))
        .route("/health/ready", axum::routing::get(readiness))
        .layer(RequestBodyLimitLayer::new(MAX_DECOMPRESSED_BODY_BYTES))
        .layer(middleware::from_fn(compressed_body_limit))
        .layer(middleware::from_fn(request_context))
        .with_state(Arc::new(state))
}

async fn compressed_body_limit(
    request: Request<axum::body::Body>,
    next: Next,
) -> impl IntoResponse {
    let encoded = request.headers().get("content-encoding").is_some();
    let too_large = request
        .headers()
        .get("content-length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| encoded && length > MAX_COMPRESSED_BODY_BYTES);
    if too_large {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError {
                code: "compressed_body_too_large",
                message: "compressed request body exceeds limit",
            }),
        )
            .into_response();
    }
    next.run(request).await
}

async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

async fn readiness(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let status = if state.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status, Json(serde_json::json!({ "ready": state.ready })))
}

async fn request_context(mut request: Request<axum::body::Body>, next: Next) -> impl IntoResponse {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .map_or_else(|| Uuid::now_v7().to_string(), ToOwned::to_owned);
    let span = info_span!("http_request", request_id = %request_id, method = %request.method(), path = %request.uri().path());
    let _entered = span.enter();
    request.extensions_mut().insert(request_id.clone());
    let mut response = next.run(request).await;
    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response.headers_mut().insert("x-request-id", value);
    }
    response
}

async fn ingest(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<IngestRequest>,
) -> impl IntoResponse {
    if request.events.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "empty_batch",
                message: "events must not be empty",
            }),
        )
            .into_response();
    }
    if request.events.len() > MAX_EVENTS_PER_BATCH {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(ApiError {
                code: "batch_too_large",
                message: "events exceeds batch limit",
            }),
        )
            .into_response();
    }
    if request
        .events
        .iter()
        .any(|event| validate_event(event).is_err())
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "invalid_event",
                message: "batch contains an invalid event",
            }),
        )
            .into_response();
    }
    if request.events.iter().any(contains_prohibited_key) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "privacy_violation",
                message: "event contains a prohibited field",
            }),
        )
            .into_response();
    }
    (
        StatusCode::ACCEPTED,
        Json(IngestAccepted {
            accepted: request.events.len(),
            duplicate: 0,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::router;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn accepts_non_empty_batch() {
        let request = Request::post("/v1/ingest")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"events":[{"schema_version":"1.0","event_type":"solana.trace.created"}]}"#,
            ))
            .unwrap();
        assert_eq!(
            router(super::AppState::default())
                .oneshot(request)
                .await
                .unwrap()
                .status(),
            StatusCode::ACCEPTED
        );
    }

    #[tokio::test]
    async fn rejects_empty_batch() {
        let request = Request::post("/v1/ingest")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"events":[]}"#))
            .unwrap();
        assert_eq!(
            router(super::AppState::default())
                .oneshot(request)
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn preserves_or_generates_request_id() {
        let request = Request::post("/v1/ingest")
            .header("content-type", "application/json")
            .header("x-request-id", "portfolio-test-1")
            .body(Body::from(
                r#"{"events":[{"schema_version":"1.0","event_type":"solana.trace.created"}]}"#,
            ))
            .unwrap();
        let response = router(super::AppState::default())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(
            response.headers().get("x-request-id").unwrap(),
            "portfolio-test-1"
        );
    }

    #[tokio::test]
    async fn liveness_and_readiness_are_distinct() {
        let app = router(super::AppState { ready: false });
        let live = app
            .clone()
            .oneshot(Request::get("/health/live").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let ready = app
            .oneshot(Request::get("/health/ready").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(live.status(), StatusCode::OK);
        assert_eq!(ready.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn rejects_oversized_compressed_body_before_json_decode() {
        let request = Request::post("/v1/ingest")
            .header("content-encoding", "gzip")
            .header("content-length", super::MAX_COMPRESSED_BODY_BYTES + 1)
            .body(Body::empty())
            .unwrap();
        let response = router(super::AppState::default())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn rejects_unsupported_event_type() {
        let request = Request::post("/v1/ingest")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"events":[{"schema_version":"1.0","event_type":"unknown"}]}"#,
            ))
            .unwrap();
        let response = router(super::AppState::default())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rejects_prohibited_privacy_key() {
        let request = Request::post("/v1/ingest")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"events":[{"schema_version":"1.0","event_type":"solana.trace.created","attributes":{"seed_phrase":"canary-secret"}}]}"#))
            .unwrap();
        let response = router(super::AppState::default())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
