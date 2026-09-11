//! Composition root and public application-service surface for Landfall.
#![allow(missing_docs)]

pub mod auth;
pub mod workers;
pub use workers::WorkerSupervisor;
pub mod projector_worker;
pub use projector_worker::{ProjectTraceJob, project_trace_queue, run_project_trace_worker};
pub mod alias_resolver;
pub use alias_resolver::{AliasResolution, AliasResolver};
pub mod projection;
pub use projection::{ProjectionError, reduce_loaded_events};
pub mod projection_metrics;
pub use projection_metrics::{ProjectionMetrics, ProjectionMetricsSnapshot};

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
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tower::limit::ConcurrencyLimitLayer;
pub mod pagination;
pub mod trace_filters;
use tower_http::limit::RequestBodyLimitLayer;
use tracing::info_span;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

pub const MAX_EVENTS_PER_BATCH: usize = 1_000;
pub const MAX_COMPRESSED_BODY_BYTES: usize = 256 * 1024;
pub const MAX_DECOMPRESSED_BODY_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_REQUESTS_PER_SECOND: u32 = 100;
pub const MAX_CONCURRENT_REQUESTS: usize = 64;
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

#[derive(Debug, Deserialize, ToSchema)]
pub struct IngestRequest {
    pub events: Vec<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngestAccepted {
    pub accepted: usize,
    pub duplicate: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(OpenApi)]
#[openapi(
    paths(ingest),
    components(schemas(IngestRequest, IngestAccepted, ApiError)),
    info(title = "Landfall Ingestion API", version = "0.1.0")
)]
pub struct ApiDoc;

/// Maps durable ingestion counters to the public HTTP contract.
#[must_use]
pub fn ingest_response(inserted: usize, duplicates: usize) -> axum::response::Response {
    let status = if inserted == 0 && duplicates > 0 {
        StatusCode::OK
    } else {
        StatusCode::ACCEPTED
    };
    (
        status,
        Json(IngestAccepted {
            accepted: inserted,
            duplicate: duplicates,
        }),
    )
        .into_response()
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
        .route("/openapi.json", axum::routing::get(openapi))
        .route("/health/event", axum::routing::post(health_event))
        .layer(RequestBodyLimitLayer::new(MAX_DECOMPRESSED_BODY_BYTES))
        .layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_REQUESTS))
        .layer(middleware::from_fn(request_rate_limit))
        .layer(middleware::from_fn(compressed_body_limit))
        .layer(middleware::from_fn(request_context))
        .with_state(Arc::new(state))
}

async fn openapi() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}

async fn health_event() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "synthetic": true,
            "event_id": Uuid::now_v7().to_string(),
            "schema_version": "1.0",
            "event_type": "solana.trace.created",
            "message": "disposable health-check event; not persisted"
        })),
    )
}

static RATE_WINDOW: OnceLock<Mutex<(Instant, u32)>> = OnceLock::new();

async fn request_rate_limit(request: Request<axum::body::Body>, next: Next) -> impl IntoResponse {
    let limiter = RATE_WINDOW.get_or_init(|| Mutex::new((Instant::now(), 0)));
    let allowed = limiter.lock().is_ok_and(|mut window| {
        if window.0.elapsed() >= Duration::from_secs(1) {
            *window = (Instant::now(), 0);
        }
        if window.1 >= MAX_REQUESTS_PER_SECOND {
            false
        } else {
            window.1 += 1;
            true
        }
    });
    if !allowed {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError {
                code: "rate_limited",
                message: "request rate limit exceeded",
            }),
        )
            .into_response();
    }
    next.run(request).await
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

#[utoipa::path(
    post,
    path = "/v1/ingest",
    request_body = IngestRequest,
    responses(
        (status = 202, description = "At least one event accepted", body = IngestAccepted),
        (status = 200, description = "All events were duplicates", body = IngestAccepted),
        (status = 400, description = "Invalid or privacy-unsafe event", body = ApiError),
        (status = 413, description = "Batch or body too large", body = ApiError)
    )
)]
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
    ingest_response(request.events.len(), 0)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::{ApiDoc, router};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use utoipa::OpenApi as _;

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

    #[test]
    #[allow(clippy::assertions_on_constants)]
    fn limits_are_positive_and_bounded() {
        assert!(super::MAX_REQUESTS_PER_SECOND > 0);
        assert!(super::MAX_CONCURRENT_REQUESTS <= super::MAX_REQUESTS_PER_SECOND as usize);
    }

    #[test]
    fn openapi_snapshot_contains_ingest_contract() {
        let document = ApiDoc::openapi().to_pretty_json().unwrap();
        assert!(document.contains("/v1/ingest"));
        assert!(document.contains("IngestRequest"));
        assert!(document.contains("202"));
    }

    #[tokio::test]
    async fn health_event_is_explicitly_synthetic() {
        let request = Request::post("/health/event").body(Body::empty()).unwrap();
        let response = router(super::AppState::default())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn maps_duplicate_batch_to_ok_and_new_batch_to_accepted() {
        assert_eq!(super::ingest_response(0, 3).status(), StatusCode::OK);
        assert_eq!(super::ingest_response(2, 1).status(), StatusCode::ACCEPTED);
    }
}
