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
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::info_span;
use uuid::Uuid;

pub const MAX_EVENTS_PER_BATCH: usize = 1_000;

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

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/ingest", post(ingest))
        .route("/health/live", axum::routing::get(liveness))
        .route("/health/ready", axum::routing::get(readiness))
        .layer(middleware::from_fn(request_context))
        .with_state(Arc::new(state))
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
            .body(Body::from(r#"{"events":[{"type":"created"}]}"#))
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
            .body(Body::from(r#"{"events":[{"type":"created"}]}"#))
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
}
