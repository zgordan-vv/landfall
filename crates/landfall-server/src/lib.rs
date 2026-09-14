//! Composition root and public application-service surface for Landfall.
#![allow(missing_docs)]

pub mod auth;
pub mod control_plane;
pub mod workers;
pub use workers::WorkerSupervisor;
pub mod observability;
pub mod observation_worker;
pub use observation_worker::{
    ObservationWorkerConfig, ObservationWorkerMetrics, ObservationWorkerMetricsSnapshot,
    run_observation_worker,
};
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
    extract::{Extension, Path, Query, State},
    http::{HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, post, put},
};
use landfall_protocol::check_event_compatibility;
use landfall_storage::{
    IngestEvent, TraceProjectionWrite, enqueue_observation_if_eligible, ensure_raw_event_partition,
    ingest_atomically, load_events_for_trace, replace_trace_projection,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use std::{path::PathBuf, sync::Arc};
use tower::limit::ConcurrencyLimitLayer;
pub mod acknowledged_events;
pub mod artifact_store;
pub mod backup;
pub mod business_action;
pub mod cohort_comparison;
pub mod collector_benchmark;
pub mod config_read_model;
pub mod dashboard_query_benchmark;
pub mod data_quality_summary;
pub mod deletion;
pub mod etag;
pub mod export_safety;
pub mod fault_injection;
pub mod fuzz_guards;
pub mod job_recovery;
pub mod log_safety;
pub mod metrics_endpoint;
pub mod metrics_summary;
pub mod observer_benchmark;
pub mod pagination;
pub mod projection_benchmark;
pub mod query_benchmark;
pub mod recommendation_disposition;
pub mod report_benchmark;
pub mod report_jobs;
pub mod retention;
pub mod retention_benchmark;
pub mod secret_matrix;
pub mod signature_lookup;
pub mod storage_benchmark;
pub mod system_status;
pub mod trace_detail;
pub mod trace_filters;
pub mod workload;
pub mod x402_payments;
use crate::auth::AuthenticatedToken;
use crate::control_plane::{
    create_environment, create_project, create_route, create_token, create_x402_spend_policy,
    disable_route, list_environments, list_routes, list_tokens, list_x402_payment_audit,
    list_x402_spend_policy, revoke_token, update_x402_spend_policy,
};
use tower_http::{
    limit::RequestBodyLimitLayer,
    services::{ServeDir, ServeFile},
};
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
    /// Optional durable PostgreSQL pool; absent only for isolated unit tests.
    pub pool: Option<sqlx::PgPool>,
    /// Hash of the deployment-local bootstrap credential for first-project provisioning.
    pub bootstrap_token_hash: Option<[u8; 32]>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct IngestRequest {
    /// Optional stable client batch identity used to make transport retries idempotent.
    pub batch_id: Option<String>,
    pub events: Vec<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IngestAccepted {
    pub accepted: usize,
    pub duplicate: usize,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TraceListItem {
    pub trace_id: String,
    pub lifecycle_state: String,
    pub landing_state: String,
    pub execution_state: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OverviewSummary {
    pub window_hours: u32,
    pub total_traces: i64,
    pub landed_traces: i64,
    pub successful_executions: i64,
    pub unknown_executions: i64,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SystemHealthSummary {
    pub status: &'static str,
    pub database_ready: bool,
    pub projects: i64,
    pub environments: i64,
    pub enabled_routes: i64,
    pub queued_jobs: i64,
    pub dead_letter_jobs: i64,
    pub events_last_24h: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ComparisonSummary {
    pub baseline_environment_id: String,
    pub candidate_environment_id: String,
    pub baseline_traces: i64,
    pub candidate_traces: i64,
    pub baseline_landed: i64,
    pub candidate_landed: i64,
}

#[derive(Debug, Deserialize)]
pub struct TraceListQuery {
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiError {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        ingest,
        control_plane::create_project,
        control_plane::create_environment,
        control_plane::list_environments,
        control_plane::create_route,
        control_plane::list_routes,
        control_plane::disable_route,
        control_plane::create_token,
        control_plane::list_tokens,
        control_plane::revoke_token,
        control_plane::create_x402_spend_policy,
        control_plane::list_x402_spend_policy,
        control_plane::list_x402_payment_audit,
        control_plane::update_x402_spend_policy,
        crate::x402_payments::authorize,
        crate::x402_payments::record_settlement
    ),
    components(schemas(
        IngestRequest,
        IngestAccepted,
        ApiError,
        crate::metrics_summary::MetricSummary,
        crate::data_quality_summary::DataQualitySummary,
        crate::cohort_comparison::CohortComparison,
        crate::recommendation_disposition::DispositionRecord,
        crate::system_status::DetailedSystemStatus,
        crate::config_read_model::ProjectConfig,
        crate::control_plane::CreateProjectRequest,
        crate::control_plane::CreatedProjectResponse,
        crate::control_plane::CreateEnvironmentRequest,
        crate::control_plane::EnvironmentResponse,
        crate::control_plane::CreateRouteRequest,
        crate::control_plane::RouteResponse,
        crate::control_plane::CreateTokenRequest,
        crate::control_plane::CreatedTokenResponse,
        crate::control_plane::TokenResponse,
        crate::control_plane::UpsertX402SpendPolicyRequest,
        crate::control_plane::X402SpendPolicyResponse,
        crate::control_plane::X402PaymentAuditResponse,
        crate::x402_payments::X402AuthorizeRequest,
        crate::x402_payments::X402AuthorizeResponse,
        crate::x402_payments::X402SettlementRequest,
        crate::x402_payments::X402SettlementResponse
    )),
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
    let state = Arc::new(state);
    let api = Router::new()
        .route("/v1/x402/authorize", post(x402_payments::authorize))
        .route(
            "/v1/x402/settlements",
            post(x402_payments::record_settlement),
        )
        .route("/v1/ingest", post(ingest))
        .route("/v1/traces/{trace_id}", get(trace_detail))
        .route("/v1/traces/{trace_id}/diagnostics", get(trace_diagnostics))
        .route(
            "/v1/traces/{trace_id}/recommendations",
            get(trace_recommendations),
        )
        .route("/v1/traces", get(trace_list))
        .route("/v1/overview", get(overview))
        .route("/v1/system/status", get(system_status))
        .route("/v1/comparison", get(comparison))
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            authenticate_api,
        ));
    let bootstrap = Router::new().route("/v1/control/projects", post(create_project));
    let control = Router::new()
        .route(
            "/v1/control/projects/{project_id}/environments",
            post(create_environment).get(list_environments),
        )
        .route(
            "/v1/control/projects/{project_id}/environments/{environment_id}/routes",
            post(create_route).get(list_routes),
        )
        .route(
            "/v1/control/projects/{project_id}/environments/{environment_id}/routes/{route_id}/disable",
            post(disable_route),
        )
        .route(
            "/v1/control/projects/{project_id}/tokens",
            post(create_token).get(list_tokens),
        )
        .route(
            "/v1/control/projects/{project_id}/tokens/{token_id}/revoke",
            post(revoke_token),
        )
        .route(
            "/v1/control/projects/{project_id}/x402/policies",
            post(create_x402_spend_policy).get(list_x402_spend_policy),
        )
        .route(
            "/v1/control/projects/{project_id}/x402/audit",
            get(list_x402_payment_audit),
        )
        .route(
            "/v1/control/projects/{project_id}/x402/policies/{policy_id}",
            put(update_x402_spend_policy),
        )
        .route_layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            authenticate_api,
        ));
    let router = Router::new()
        .merge(api)
        .merge(bootstrap)
        .merge(control)
        .route("/health/live", axum::routing::get(liveness))
        .route("/health/ready", axum::routing::get(readiness))
        .route("/metrics", axum::routing::get(metrics))
        .route("/openapi.json", axum::routing::get(openapi))
        .route("/health/event", axum::routing::post(health_event))
        .layer(RequestBodyLimitLayer::new(MAX_DECOMPRESSED_BODY_BYTES))
        .layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_REQUESTS))
        .layer(middleware::from_fn(request_rate_limit))
        .layer(middleware::from_fn(compressed_body_limit))
        .layer(middleware::from_fn(request_context))
        .with_state(state);
    dashboard_router(router).layer(middleware::from_fn(security_headers_and_cors))
}

fn dashboard_router(router: Router) -> Router {
    let directory = std::env::var_os("LANDFALL_DASHBOARD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/landfall/dashboard"));
    dashboard_router_at(router, directory)
}

fn dashboard_router_at(router: Router, directory: PathBuf) -> Router {
    let index = directory.join("index.html");
    if index.is_file() {
        router.fallback_service(ServeDir::new(directory).not_found_service(ServeFile::new(index)))
    } else {
        router
    }
}

fn required_scope(path: &str) -> &'static str {
    if path.starts_with("/v1/control/") {
        "project:admin"
    } else if path == "/v1/x402/authorize" || path == "/v1/x402/settlements" {
        "x402:pay"
    } else if path == "/v1/ingest" {
        "ingest:write"
    } else if path == "/v1/system/status" {
        "admin"
    } else if path.contains("diagnostics") || path.contains("recommendations") {
        "diagnostics:read"
    } else {
        "traces:read"
    }
}

async fn authenticate_api(
    State(state): State<Arc<AppState>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return next.run(request).await;
    };
    match auth::authenticate_database(
        pool,
        request
            .headers()
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        required_scope(request.uri().path()),
    )
    .await
    {
        Ok(principal) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(auth::AuthError::MissingScope) => (
            StatusCode::FORBIDDEN,
            Json(ApiError {
                code: "insufficient_scope",
                message: "token does not grant this operation",
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                code: "unauthorized",
                message: "valid bearer authentication is required",
            }),
        )
            .into_response(),
    }
}

async fn security_headers_and_cors(
    request: Request<axum::body::Body>,
    next: Next,
) -> impl IntoResponse {
    let origin = request
        .headers()
        .get("origin")
        .and_then(|value| value.to_str().ok());
    let host = request
        .headers()
        .get("host")
        .and_then(|value| value.to_str().ok());
    if origin.is_some_and(|value| !cors_allows_origin(value, host)) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiError {
                code: "origin_not_allowed",
                message: "cross-origin request is not allowed",
            }),
        )
            .into_response();
    }
    let is_dashboard = is_dashboard_request(request.uri().path());
    let mut response = next.run(request).await.into_response();
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "content-security-policy",
        if is_dashboard {
            HeaderValue::from_static("default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'")
        } else {
            HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'")
        },
    );
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    response
}

fn is_dashboard_request(path: &str) -> bool {
    !path.starts_with("/v1/")
        && !matches!(
            path,
            "/health/live" | "/health/ready" | "/health/event" | "/metrics" | "/openapi.json"
        )
}

fn cors_allows_origin(origin: &str, host: Option<&str>) -> bool {
    origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .is_some_and(|value| host.is_some_and(|expected| value == expected))
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

#[derive(Debug)]
struct RateWindow {
    started_at: Instant,
    total_requests: u32,
}

static RATE_WINDOW: OnceLock<Mutex<RateWindow>> = OnceLock::new();

async fn request_rate_limit(request: Request<axum::body::Body>, next: Next) -> impl IntoResponse {
    let limiter = RATE_WINDOW.get_or_init(|| {
        Mutex::new(RateWindow {
            started_at: Instant::now(),
            total_requests: 0,
        })
    });
    let allowed = limiter.lock().is_ok_and(|mut window| {
        if window.started_at.elapsed() >= Duration::from_secs(1) {
            *window = RateWindow {
                started_at: Instant::now(),
                total_requests: 0,
            };
        }
        if window.total_requests >= MAX_REQUESTS_PER_SECOND {
            false
        } else {
            window.total_requests += 1;
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

async fn metrics(State(state): State<Arc<AppState>>) -> axum::response::Response {
    let durable = match state.pool.as_ref() {
        Some(pool) => metrics_endpoint::load(pool).await.ok(),
        None => None,
    };
    let status = if durable.is_some() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics_endpoint::render(state.ready, durable),
    )
        .into_response()
}

async fn trace_detail(
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(trace_id): Path<String>,
) -> impl IntoResponse {
    let Ok(trace_uuid) = Uuid::parse_str(&trace_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "invalid_trace_id",
                message: "trace_id must be a UUID",
            }),
        )
            .into_response();
    };
    let Some(pool) = state.pool.as_ref() else {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiError {
                code: "trace_not_found",
                message: "trace was not found",
            }),
        )
            .into_response();
    };
    let row = sqlx::query("SELECT trace_id, lifecycle_state, landing_state, execution_state, application_state, observation_state, updated_at FROM reporting.traces WHERE trace_id = $1 AND ($2::uuid IS NULL OR project_id = $2)")
        .bind(trace_uuid).bind(principal.as_ref().map(|Extension(value)| value.project_id)).fetch_optional(pool).await;
    match row {
        Ok(Some(row)) => (StatusCode::OK, Json(serde_json::json!({
            "trace_id": row.get::<Uuid, _>("trace_id").to_string(),
            "lifecycle_state": row.get::<String, _>("lifecycle_state"),
            "landing_state": row.get::<String, _>("landing_state"),
            "execution_state": row.get::<String, _>("execution_state"),
            "application_state": row.get::<String, _>("application_state"),
            "observation_state": row.get::<String, _>("observation_state"),
            "updated_at": row.get::<time::OffsetDateTime, _>("updated_at").format(&time::format_description::well_known::Rfc3339).unwrap_or_default(),
        }))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(ApiError { code: "trace_not_found", message: "trace was not found" })).into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { code: "storage_unavailable", message: "trace query failed" })).into_response(),
    }
}

async fn trace_diagnostics(
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(trace_id): Path<String>,
) -> impl IntoResponse {
    let Ok(trace_uuid) = Uuid::parse_str(&trace_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "invalid_trace_id",
                message: "trace_id must be a UUID",
            }),
        )
            .into_response();
    };
    let Some(pool) = state.pool.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "diagnostics requires durable storage",
            }),
        )
            .into_response();
    };
    let rows = sqlx::query("SELECT d.diagnostic_id, d.rule_id, d.claim_key, d.certainty FROM reporting.diagnostics d JOIN reporting.traces t ON t.trace_id = d.trace_id WHERE d.trace_id = $1 AND ($2::uuid IS NULL OR t.project_id = $2) ORDER BY d.created_at DESC").bind(trace_uuid).bind(principal.as_ref().map(|Extension(value)| value.project_id)).fetch_all(pool).await;
    match rows { Ok(rows) => (StatusCode::OK, Json(rows.into_iter().map(|row| serde_json::json!({"diagnostic_id": row.get::<Uuid, _>("diagnostic_id").to_string(), "rule_id": row.get::<String, _>("rule_id"), "claim_key": row.get::<String, _>("claim_key"), "certainty": row.get::<String, _>("certainty")})).collect::<Vec<_>>())).into_response(), Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { code: "storage_unavailable", message: "diagnostics query failed" })).into_response() }
}

async fn trace_list(
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Query(query): Query<TraceListQuery>,
) -> impl IntoResponse {
    let Some(pool) = state.pool.as_ref() else {
        return (StatusCode::OK, Json(Vec::<TraceListItem>::new())).into_response();
    };
    let limit = query.limit.unwrap_or(50).clamp(1, 100) as i64;
    let rows = sqlx::query("SELECT trace_id, lifecycle_state, landing_state, execution_state, updated_at FROM reporting.traces WHERE ($1::uuid IS NULL OR project_id = $1) ORDER BY updated_at DESC, trace_id DESC LIMIT $2")
        .bind(principal.as_ref().map(|Extension(value)| value.project_id)).bind(limit).fetch_all(pool).await;
    match rows {
        Ok(rows) => (
            StatusCode::OK,
            Json(
                rows.into_iter()
                    .map(|row| TraceListItem {
                        trace_id: row.get::<Uuid, _>("trace_id").to_string(),
                        lifecycle_state: row.get("lifecycle_state"),
                        landing_state: row.get("landing_state"),
                        execution_state: row.get("execution_state"),
                        updated_at: row
                            .get::<time::OffsetDateTime, _>("updated_at")
                            .format(&time::format_description::well_known::Rfc3339)
                            .unwrap_or_default(),
                    })
                    .collect::<Vec<_>>(),
            ),
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "trace list query failed",
            }),
        )
            .into_response(),
    }
}

async fn overview(
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
) -> impl IntoResponse {
    let Some(pool) = state.pool.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "overview requires durable storage",
            }),
        )
            .into_response();
    };
    let row = sqlx::query("SELECT COUNT(*)::bigint AS total_traces, COUNT(*) FILTER (WHERE landing_state = 'landed')::bigint AS landed_traces, COUNT(*) FILTER (WHERE execution_state = 'success')::bigint AS successful_executions, COUNT(*) FILTER (WHERE execution_state = 'unknown')::bigint AS unknown_executions, COALESCE(MAX(updated_at), now()) AS updated_at FROM reporting.traces WHERE updated_at >= now() - interval '24 hours' AND ($1::uuid IS NULL OR project_id = $1)")
        .bind(principal.as_ref().map(|Extension(value)| value.project_id)).fetch_one(pool).await;
    match row {
        Ok(row) => (
            StatusCode::OK,
            Json(OverviewSummary {
                window_hours: 24,
                total_traces: row.get("total_traces"),
                landed_traces: row.get("landed_traces"),
                successful_executions: row.get("successful_executions"),
                unknown_executions: row.get("unknown_executions"),
                updated_at: row
                    .get::<time::OffsetDateTime, _>("updated_at")
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "overview query failed",
            }),
        )
            .into_response(),
    }
}

async fn system_status(
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
) -> impl IntoResponse {
    let Some(pool) = state.pool.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "system status requires durable storage",
            }),
        )
            .into_response();
    };
    let project_id = principal.as_ref().map(|Extension(value)| value.project_id);
    let row = sqlx::query("SELECT (SELECT COUNT(*)::bigint FROM control.projects WHERE project_id = $1) AS projects, (SELECT COUNT(*)::bigint FROM control.environments WHERE project_id = $1) AS environments, (SELECT COUNT(*)::bigint FROM control.routes r JOIN control.environments e ON e.environment_id = r.environment_id WHERE r.enabled AND e.project_id = $1) AS enabled_routes, (SELECT COUNT(*)::bigint FROM work.jobs j JOIN reporting.traces t ON t.trace_id = (j.payload ->> 'trace_id')::uuid WHERE j.status IN ('ready', 'running') AND t.project_id = $1) AS queued_jobs, (SELECT COUNT(*)::bigint FROM work.jobs j JOIN reporting.traces t ON t.trace_id = (j.payload ->> 'trace_id')::uuid WHERE j.status = 'dead_letter' AND t.project_id = $1) AS dead_letter_jobs, (SELECT COUNT(*)::bigint FROM telemetry.raw_events WHERE received_at >= now() - interval '24 hours' AND project_id = $1) AS events_last_24h")
        .bind(project_id).fetch_one(pool).await;
    match row {
        Ok(row) => {
            let projects = row.get::<i64, _>("projects");
            let environments = row.get::<i64, _>("environments");
            let enabled_routes = row.get::<i64, _>("enabled_routes");
            let queued_jobs = row.get::<i64, _>("queued_jobs");
            let dead_letter_jobs = row.get::<i64, _>("dead_letter_jobs");
            let events_last_24h = row.get::<i64, _>("events_last_24h");
            let status = if projects > 0 && environments > 0 && dead_letter_jobs == 0 {
                "ok"
            } else {
                "degraded"
            };
            (
                StatusCode::OK,
                Json(SystemHealthSummary {
                    status,
                    database_ready: true,
                    projects,
                    environments,
                    enabled_routes,
                    queued_jobs,
                    dead_letter_jobs,
                    events_last_24h,
                }),
            )
                .into_response()
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "system status query failed",
            }),
        )
            .into_response(),
    }
}

async fn trace_recommendations(
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(trace_id): Path<String>,
) -> impl IntoResponse {
    let Ok(trace_uuid) = Uuid::parse_str(&trace_id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                code: "invalid_trace_id",
                message: "trace_id must be a UUID",
            }),
        )
            .into_response();
    };
    let Some(pool) = state.pool.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "recommendations require durable storage",
            }),
        )
            .into_response();
    };
    match sqlx::query("SELECT r.recommendation_id, r.recommendation_key, r.rule_set_version FROM reporting.recommendations r JOIN reporting.traces t ON t.trace_id = r.trace_id WHERE r.trace_id = $1 AND ($2::uuid IS NULL OR t.project_id = $2) ORDER BY r.created_at DESC").bind(trace_uuid).bind(principal.as_ref().map(|Extension(value)| value.project_id)).fetch_all(pool).await {
        Ok(rows) => (StatusCode::OK, Json(rows.into_iter().map(|row| serde_json::json!({"recommendation_id": row.get::<Uuid, _>("recommendation_id").to_string(), "recommendation_key": row.get::<String, _>("recommendation_key"), "rule_set_version": row.get::<String, _>("rule_set_version")})).collect::<Vec<_>>())).into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { code: "storage_unavailable", message: "recommendations query failed" })).into_response(),
    }
}

async fn comparison(
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
) -> impl IntoResponse {
    let Some(pool) = state.pool.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "comparison requires durable storage",
            }),
        )
            .into_response();
    };
    let rows = sqlx::query("SELECT environment_id, COUNT(*)::bigint AS traces, COUNT(*) FILTER (WHERE landing_state = 'landed')::bigint AS landed FROM reporting.traces WHERE ($1::uuid IS NULL OR project_id = $1) GROUP BY environment_id ORDER BY MAX(updated_at) DESC, environment_id DESC LIMIT 2").bind(principal.as_ref().map(|Extension(value)| value.project_id)).fetch_all(pool).await;
    match rows {
        Ok(rows) if rows.len() == 2 => (
            StatusCode::OK,
            Json(ComparisonSummary {
                baseline_environment_id: rows[1].get::<Uuid, _>("environment_id").to_string(),
                candidate_environment_id: rows[0].get::<Uuid, _>("environment_id").to_string(),
                baseline_traces: rows[1].get("traces"),
                candidate_traces: rows[0].get("traces"),
                baseline_landed: rows[1].get("landed"),
                candidate_landed: rows[0].get("landed"),
            }),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ApiError {
                code: "insufficient_cohorts",
                message: "comparison requires traces in two environments",
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "comparison query failed",
            }),
        )
            .into_response(),
    }
}

fn state_label<T: std::fmt::Debug>(value: T) -> String {
    format!("{value:?}")
        .chars()
        .enumerate()
        .fold(String::new(), |mut out, (index, character)| {
            if character.is_uppercase() && index > 0 {
                out.push('_');
            }
            out.push(character.to_ascii_lowercase());
            out
        })
}

pub async fn refresh_trace_projection(
    pool: &sqlx::PgPool,
    project_id: Uuid,
    environment_id: Uuid,
    trace_id: Uuid,
) -> Result<(), ()> {
    let from = time::OffsetDateTime::UNIX_EPOCH;
    let until = time::OffsetDateTime::now_utc() + time::Duration::days(1);
    let rows = load_events_for_trace(pool, project_id, environment_id, trace_id, from, until)
        .await
        .map_err(|_| ())?;
    let projection = reduce_loaded_events(rows).map_err(|_| ())?;
    let trace = projection.trace();
    let state = trace.state();
    replace_trace_projection(
        pool,
        &TraceProjectionWrite {
            trace_id: trace.id().into_uuid(),
            project_id: projection.project_id().into_uuid(),
            environment_id: trace.environment_id().into_uuid(),
            lifecycle_state: state_label(state.lifecycle),
            landing_state: state_label(state.landing),
            execution_state: state_label(state.execution),
            application_state: state_label(state.application),
            observation_state: state_label(state.observation),
            updated_at: time::OffsetDateTime::now_utc(),
            attempts: Vec::new(),
        },
    )
    .await
    .map_err(|_| ())
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
    tracing::info!(
        status = response.status().as_u16(),
        "http_request_completed"
    );
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
    State(state): State<Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
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
    let Some(pool) = state.pool.as_ref() else {
        return ingest_response(request.events.len(), 0);
    };
    let batch_id = match request.batch_id {
        Some(value) => match Uuid::parse_str(&value) {
            Ok(value) => value,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiError {
                        code: "invalid_batch_id",
                        message: "batch_id must be a UUID",
                    }),
                )
                    .into_response();
            }
        },
        None => Uuid::now_v7(),
    };
    let mut durable = Vec::with_capacity(request.events.len());
    for event in request.events {
        let object = event.as_object().expect("validated event object");
        let parse_uuid = |key: &str| {
            object
                .get(key)
                .and_then(Value::as_str)
                .and_then(|v| Uuid::parse_str(v).ok())
        };
        let Some(event_id) = parse_uuid("event_id") else {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    code: "invalid_event_id",
                    message: "event_id must be a UUID",
                }),
            )
                .into_response();
        };
        let Some(project_id) = parse_uuid("project_id") else {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    code: "invalid_project_id",
                    message: "project_id must be a UUID",
                }),
            )
                .into_response();
        };
        if principal
            .as_ref()
            .is_some_and(|Extension(token)| token.project_id != project_id)
        {
            return (
                StatusCode::FORBIDDEN,
                Json(ApiError {
                    code: "project_forbidden",
                    message: "token cannot ingest for this project",
                }),
            )
                .into_response();
        }
        let Some(environment_id) = parse_uuid("environment_id") else {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    code: "invalid_environment_id",
                    message: "environment_id must be a UUID",
                }),
            )
                .into_response();
        };
        let Some(occurred_at) = object
            .get("occurred_at")
            .and_then(Value::as_str)
            .and_then(|v| {
                time::OffsetDateTime::parse(v, &time::format_description::well_known::Rfc3339).ok()
            })
        else {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    code: "invalid_occurred_at",
                    message: "occurred_at must be RFC3339",
                }),
            )
                .into_response();
        };
        let event_type = object
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let payload_bytes = serde_json::to_vec(&event).unwrap_or_default();
        durable.push(IngestEvent {
            event_id,
            project_id,
            environment_id,
            trace_id: parse_uuid("trace_id"),
            event_type,
            occurred_at,
            payload: event,
            payload_hash: Sha256::digest(&payload_bytes).to_vec(),
        });
    }
    for event in &durable {
        if ensure_raw_event_partition(pool, event.occurred_at.date())
            .await
            .is_err()
        {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(ApiError {
                    code: "storage_unavailable",
                    message: "durable ingestion failed",
                }),
            )
                .into_response();
        }
    }
    for event in &durable {
        if let Some(trace_id) = event.trace_id {
            if sqlx::query("INSERT INTO reporting.traces (trace_id, project_id, environment_id) VALUES ($1, $2, $3) ON CONFLICT (trace_id) DO NOTHING")
                .bind(trace_id).bind(event.project_id).bind(event.environment_id)
                .execute(pool).await.is_err() {
                return (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { code: "storage_unavailable", message: "durable ingestion failed" })).into_response();
            }
        }
    }
    match ingest_atomically(pool, batch_id, &durable).await {
        Ok(outcome) => {
            let mut projected = std::collections::HashSet::new();
            for event in &durable {
                if let Some(trace_id) = event.trace_id {
                    if projected.insert(trace_id)
                        && refresh_trace_projection(
                            pool,
                            event.project_id,
                            event.environment_id,
                            trace_id,
                        )
                        .await
                        .is_err()
                    {
                        return (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { code: "projection_unavailable", message: "event was durable but its trace projection could not be refreshed" })).into_response();
                    }
                    if enqueue_observation_if_eligible(pool, trace_id, true)
                        .await
                        .is_err()
                    {
                        return (StatusCode::SERVICE_UNAVAILABLE, Json(ApiError { code: "observation_queue_unavailable", message: "trace was durable but could not be queued for observation" })).into_response();
                    }
                }
            }
            ingest_response(outcome.inserted, outcome.duplicates)
        }
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiError {
                code: "storage_unavailable",
                message: "durable ingestion failed",
            }),
        )
            .into_response(),
    }
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
        let app = router(super::AppState {
            ready: false,
            pool: None,
            bootstrap_token_hash: None,
        });
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
        assert!(document.contains("/v1/control/projects"));
        assert!(document.contains("IngestRequest"));
        assert!(document.contains("202"));
    }

    #[test]
    fn cors_policy_allows_only_same_origin_host() {
        assert!(super::cors_allows_origin(
            "https://example.test",
            Some("example.test")
        ));
        assert!(!super::cors_allows_origin(
            "https://evil.test",
            Some("example.test")
        ));
        assert!(!super::cors_allows_origin("null", Some("example.test")));
    }

    #[test]
    fn route_scope_classification_is_explicit() {
        assert_eq!(
            super::required_scope("/v1/control/projects/id/tokens"),
            "project:admin"
        );
        assert_eq!(super::required_scope("/v1/ingest"), "ingest:write");
        assert_eq!(super::required_scope("/v1/traces"), "traces:read");
        assert_eq!(
            super::required_scope("/v1/traces/id/diagnostics"),
            "diagnostics:read"
        );
        assert_eq!(super::required_scope("/v1/system/status"), "admin");
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

    #[tokio::test]
    async fn metrics_report_database_unavailability_without_authentication() {
        let response = router(super::AppState::default())
            .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/plain; version=0.0.4; charset=utf-8"
        );
    }

    #[tokio::test]
    async fn dashboard_bundle_is_served_without_taking_over_api_routes() {
        let directory =
            std::env::temp_dir().join(format!("landfall-dashboard-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(directory.join("assets")).unwrap();
        std::fs::write(directory.join("index.html"), "<main>Landfall</main>").unwrap();
        std::fs::write(directory.join("assets/app.js"), "console.log('live')").unwrap();

        let dashboard = super::dashboard_router_at(axum::Router::new(), directory.clone())
            .layer(axum::middleware::from_fn(super::security_headers_and_cors));
        let index = dashboard
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(index.status(), StatusCode::OK);
        assert_eq!(
            index.headers().get("content-security-policy").unwrap(),
            "default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'"
        );
        assert_eq!(
            axum::body::to_bytes(index.into_body(), usize::MAX)
                .await
                .unwrap(),
            "<main>Landfall</main>"
        );
        let asset = dashboard
            .oneshot(Request::get("/assets/app.js").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(asset.status(), StatusCode::OK);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn dashboard_and_api_responses_receive_separate_content_security_policies() {
        let dashboard = router(super::AppState::default())
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            dashboard.headers().get("content-security-policy").unwrap(),
            "default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; base-uri 'none'; frame-ancestors 'none'"
        );
        let api = router(super::AppState::default())
            .oneshot(Request::get("/openapi.json").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            api.headers().get("content-security-policy").unwrap(),
            "default-src 'none'; frame-ancestors 'none'"
        );
    }

    #[tokio::test]
    async fn project_bootstrap_is_disabled_without_a_deployment_secret() {
        let request = Request::post("/v1/control/projects")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"name":"test","initial_token_name":"owner"}"#,
            ))
            .unwrap();
        let response = router(super::AppState::default())
            .oneshot(request)
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn maps_duplicate_batch_to_ok_and_new_batch_to_accepted() {
        assert_eq!(super::ingest_response(0, 3).status(), StatusCode::OK);
        assert_eq!(super::ingest_response(2, 1).status(), StatusCode::ACCEPTED);
    }
}
