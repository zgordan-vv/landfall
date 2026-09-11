//! Composition root and public application-service surface for Landfall.
#![allow(missing_docs)]

use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

pub const MAX_EVENTS_PER_BATCH: usize = 1_000;

#[derive(Clone, Default)]
pub struct AppState;

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
        .with_state(Arc::new(state))
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
            router(super::AppState)
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
            router(super::AppState)
                .oneshot(request)
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
}
