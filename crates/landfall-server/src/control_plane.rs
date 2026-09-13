//! Authenticated project, environment, and API-token provisioning endpoints.

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use getrandom::fill;
use landfall_storage::{X402SpendPolicyRecord, list_x402_spend_policies, upsert_x402_spend_policy};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, AppState, AuthenticatedToken, auth};

const ADMIN_SCOPE: &str = "project:admin";
const ALLOWED_SCOPES: &[&str] = &[
    "project:admin",
    "ingest:write",
    "traces:read",
    "diagnostics:read",
    "admin",
    "x402:pay",
];

/// Request to provision a new customer project and its first administrator token.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    /// Human-readable project name.
    pub name: String,
    /// Label shown in token listings for the initial administrator token.
    pub initial_token_name: String,
}

/// Project identity returned by the control plane.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProjectResponse {
    pub project_id: String,
    pub name: String,
}

/// Response emitted when a plaintext token is created. The `token` value is not
/// retrievable later; PostgreSQL stores only its SHA-256 digest.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedTokenResponse {
    pub token_id: String,
    pub name: String,
    pub token_prefix: String,
    pub token: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
}

/// Atomic bootstrap response containing a new project and its first token.
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedProjectResponse {
    pub project: ProjectResponse,
    pub initial_token: CreatedTokenResponse,
}

/// Request to add an environment to a project.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEnvironmentRequest {
    pub name: String,
    pub cluster: String,
}

/// Durable environment metadata.
#[derive(Debug, Serialize, ToSchema)]
pub struct EnvironmentResponse {
    pub environment_id: String,
    pub project_id: String,
    pub name: String,
    pub cluster: String,
}

/// Request to register one customer-controlled Solana JSON-RPC route.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRouteRequest {
    pub name: String,
    /// HTTPS URL used only by the observer worker; it is never returned by listing APIs.
    pub endpoint: String,
}

/// Non-secret route metadata. Endpoint URLs are intentionally omitted.
#[derive(Debug, Serialize, ToSchema)]
pub struct RouteResponse {
    pub route_id: String,
    pub environment_id: String,
    pub name: String,
    pub enabled: bool,
}

/// Request to mint a scoped token for one project.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scopes: Vec<String>,
    /// Optional RFC 3339 expiry; omitted tokens do not expire automatically.
    pub expires_at: Option<String>,
}

/// Non-secret token metadata for token listing and revocation workflows.
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub token_id: String,
    pub name: String,
    pub token_prefix: String,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// Request to create or update a non-custodial x402 spend policy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertX402SpendPolicyRequest {
    /// Stable application-owned agent label.
    pub agent_id: String,
    /// CAIP-2 network identifier, such as `solana:mainnet`.
    pub network: String,
    /// Asset identifier selected by the deployment.
    pub asset: String,
    /// Canonical positive base-10 atomic amount.
    pub max_per_request_atomic: String,
    /// Canonical positive base-10 aggregate daily cap.
    pub max_per_day_atomic: String,
    /// Exact HTTPS origins that the agent may pay.
    pub merchant_origins: Vec<String>,
    /// Whether the policy may authorize future requests.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// Safe x402 policy data returned to the project owner.
#[derive(Debug, Serialize, ToSchema)]
pub struct X402SpendPolicyResponse {
    pub policy_id: String,
    pub project_id: String,
    pub agent_id: String,
    pub network: String,
    pub asset: String,
    pub max_per_request_atomic: String,
    pub max_per_day_atomic: String,
    pub merchant_origins: Vec<String>,
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn error(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> axum::response::Response {
    (status, Json(ApiError { code, message })).into_response()
}

fn valid_text(value: &str, maximum: usize) -> bool {
    let length = value.chars().count();
    (1..=maximum).contains(&length)
}

fn valid_scopes(scopes: &[String]) -> bool {
    !scopes.is_empty()
        && scopes.len() <= ALLOWED_SCOPES.len()
        && scopes
            .iter()
            .all(|scope| ALLOWED_SCOPES.contains(&scope.as_str()))
        && scopes
            .iter()
            .enumerate()
            .all(|(index, scope)| !scopes[..index].iter().any(|previous| previous == scope))
}

fn valid_rpc_endpoint(endpoint: &str) -> bool {
    endpoint.len() <= 2048
        && endpoint.starts_with("https://")
        && !endpoint.contains('@')
        && !endpoint.chars().any(char::is_whitespace)
}

fn valid_atomic_amount(value: &str) -> bool {
    !value.is_empty()
        && value != "0"
        && !(value.len() > 1 && value.starts_with('0'))
        && value.parse::<u128>().is_ok()
}

fn valid_merchant_origin(value: &str) -> bool {
    let Some(host) = value.strip_prefix("https://") else {
        return false;
    };
    !host.is_empty()
        && host.len() <= 2040
        && !host.contains(['/', '?', '#', '@'])
        && !host.chars().any(char::is_whitespace)
        && value == value.to_ascii_lowercase()
}

fn valid_x402_policy(request: &UpsertX402SpendPolicyRequest) -> bool {
    let limits_are_ordered = request
        .max_per_request_atomic
        .parse::<u128>()
        .ok()
        .zip(request.max_per_day_atomic.parse::<u128>().ok())
        .is_some_and(|(per_request, per_day)| per_request <= per_day);
    valid_text(&request.agent_id, 160)
        && valid_text(&request.network, 160)
        && valid_text(&request.asset, 256)
        && valid_atomic_amount(&request.max_per_request_atomic)
        && valid_atomic_amount(&request.max_per_day_atomic)
        && limits_are_ordered
        && !request.merchant_origins.is_empty()
        && request.merchant_origins.len() <= 100
        && request
            .merchant_origins
            .iter()
            .all(|origin| valid_merchant_origin(origin))
        && request
            .merchant_origins
            .iter()
            .enumerate()
            .all(|(index, origin)| {
                !request.merchant_origins[..index]
                    .iter()
                    .any(|previous| previous == origin)
            })
}

fn x402_response(record: X402SpendPolicyRecord) -> X402SpendPolicyResponse {
    X402SpendPolicyResponse {
        policy_id: record.policy_id.to_string(),
        project_id: record.project_id.to_string(),
        agent_id: record.agent_id,
        network: record.network,
        asset: record.asset,
        max_per_request_atomic: record.max_per_request_atomic,
        max_per_day_atomic: record.max_per_day_atomic,
        merchant_origins: record.merchant_origins,
        enabled: record.enabled,
    }
}

fn format_time(value: Option<OffsetDateTime>) -> Option<String> {
    value.and_then(|time| time.format(&Rfc3339).ok())
}

fn issue_token() -> Result<(String, String), ()> {
    let mut random = [0_u8; 32];
    fill(&mut random).map_err(|_| ())?;
    let encoded = random
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let token = format!("lf_{encoded}");
    let prefix = token[..11].to_owned();
    Ok((token, prefix))
}

fn authorize_bootstrap(headers: &HeaderMap, state: &AppState) -> bool {
    let Some(token_hash) = state.bootstrap_token_hash else {
        return false;
    };
    let record = auth::ApiTokenRecord {
        project_id: Uuid::nil(),
        token_hash,
        scopes: vec![ADMIN_SCOPE.to_owned()],
        expires_at: None,
        revoked_at: None,
    };
    auth::authorize(
        headers
            .get("authorization")
            .and_then(|value| value.to_str().ok()),
        &[record],
        ADMIN_SCOPE,
        OffsetDateTime::now_utc(),
    )
    .is_ok()
}

fn project_admin(principal: Option<Extension<AuthenticatedToken>>, project_id: Uuid) -> bool {
    principal.is_some_and(|Extension(principal)| {
        principal.project_id == project_id
            && principal.scopes.iter().any(|scope| scope == ADMIN_SCOPE)
    })
}

/// Creates a project-scoped x402 spend policy.
#[utoipa::path(post, path = "/v1/control/projects/{project_id}/x402/policies", params(("project_id" = String, Path)), request_body = UpsertX402SpendPolicyRequest, responses((status = 201, body = X402SpendPolicyResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 409, body = ApiError)))]
pub async fn create_x402_spend_policy(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(project_id): Path<Uuid>,
    Json(request): Json<UpsertX402SpendPolicyRequest>,
) -> axum::response::Response {
    upsert_x402_policy(
        state,
        principal,
        project_id,
        Uuid::now_v7(),
        request,
        StatusCode::CREATED,
    )
    .await
}

/// Replaces mutable limits, enabled state, and complete merchant allowlist of one policy.
#[utoipa::path(put, path = "/v1/control/projects/{project_id}/x402/policies/{policy_id}", params(("project_id" = String, Path), ("policy_id" = String, Path)), request_body = UpsertX402SpendPolicyRequest, responses((status = 200, body = X402SpendPolicyResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 409, body = ApiError)))]
pub async fn update_x402_spend_policy(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path((project_id, policy_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpsertX402SpendPolicyRequest>,
) -> axum::response::Response {
    upsert_x402_policy(
        state,
        principal,
        project_id,
        policy_id,
        request,
        StatusCode::OK,
    )
    .await
}

/// Lists every x402 policy owned by the authenticated project.
#[utoipa::path(get, path = "/v1/control/projects/{project_id}/x402/policies", params(("project_id" = String, Path)), responses((status = 200, body = [X402SpendPolicyResponse]), (status = 401, body = ApiError), (status = 403, body = ApiError)))]
pub async fn list_x402_spend_policy(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(project_id): Path<Uuid>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "x402 policy listing requires durable storage",
        );
    };
    match list_x402_spend_policies(pool, project_id).await {
        Ok(records) => (
            StatusCode::OK,
            Json(records.into_iter().map(x402_response).collect::<Vec<_>>()),
        )
            .into_response(),
        Err(_) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "x402 policy listing failed",
        ),
    }
}

async fn upsert_x402_policy(
    state: std::sync::Arc<AppState>,
    principal: Option<Extension<AuthenticatedToken>>,
    project_id: Uuid,
    policy_id: Uuid,
    request: UpsertX402SpendPolicyRequest,
    status: StatusCode,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    if !valid_x402_policy(&request) {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_x402_policy",
            "policy requires canonical limits and unique lowercase HTTPS merchant origins",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "x402 policy changes require durable storage",
        );
    };
    let record = X402SpendPolicyRecord {
        policy_id,
        project_id,
        agent_id: request.agent_id,
        network: request.network,
        asset: request.asset,
        max_per_request_atomic: request.max_per_request_atomic,
        max_per_day_atomic: request.max_per_day_atomic,
        merchant_origins: request.merchant_origins,
        enabled: request.enabled,
    };
    match upsert_x402_spend_policy(pool, &record).await {
        Ok(()) => (status, Json(x402_response(record))).into_response(),
        Err(_) => error(
            StatusCode::CONFLICT,
            "x402_policy_conflict",
            "policy identity conflicts with an existing policy",
        ),
    }
}

/// Creates a project and issues its first `project:admin` token in one transaction.
#[utoipa::path(post, path = "/v1/control/projects", request_body = CreateProjectRequest, responses((status = 201, body = CreatedProjectResponse), (status = 401, body = ApiError), (status = 503, body = ApiError)))]
pub async fn create_project(
    State(state): State<std::sync::Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<CreateProjectRequest>,
) -> axum::response::Response {
    if state.bootstrap_token_hash.is_none() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "bootstrap_unavailable",
            "bootstrap token is not configured",
        );
    }
    if !authorize_bootstrap(&headers, &state) {
        return error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "valid bootstrap bearer authentication is required",
        );
    }
    if !valid_text(&request.name, 200) || !valid_text(&request.initial_token_name, 120) {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_control_input",
            "project and token names must be non-empty and within their limits",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "project provisioning requires durable storage",
        );
    };
    let Ok((token, token_prefix)) = issue_token() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "token_generation_failed",
            "could not generate a secure API token",
        );
    };
    let project_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let mut transaction = match pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "storage_unavailable",
                "project provisioning failed",
            );
        }
    };
    let result = async {
        sqlx::query("INSERT INTO control.projects (project_id, name) VALUES ($1, $2)")
            .bind(project_id).bind(&request.name).execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO control.api_tokens (token_id, project_id, name, token_prefix, token_hash, scopes) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(token_id).bind(project_id).bind(&request.initial_token_name).bind(&token_prefix).bind(auth::hash_token(&token).to_vec()).bind(vec![ADMIN_SCOPE]).execute(&mut *transaction).await
    }.await;
    if result.is_err() || transaction.commit().await.is_err() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "project provisioning failed",
        );
    }
    (
        StatusCode::CREATED,
        Json(CreatedProjectResponse {
            project: ProjectResponse {
                project_id: project_id.to_string(),
                name: request.name,
            },
            initial_token: CreatedTokenResponse {
                token_id: token_id.to_string(),
                name: request.initial_token_name,
                token_prefix,
                token,
                scopes: vec![ADMIN_SCOPE.to_owned()],
                expires_at: None,
            },
        }),
    )
        .into_response()
}

/// Adds an environment to the caller's project.
#[utoipa::path(post, path = "/v1/control/projects/{project_id}/environments", params(("project_id" = String, Path)), request_body = CreateEnvironmentRequest, responses((status = 201, body = EnvironmentResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError)))]
pub async fn create_environment(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(project_id): Path<Uuid>,
    Json(request): Json<CreateEnvironmentRequest>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    if !valid_text(&request.name, 120) || !valid_text(&request.cluster, 80) {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_control_input",
            "environment name and cluster must be non-empty and within their limits",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "environment provisioning requires durable storage",
        );
    };
    let environment_id = Uuid::now_v7();
    match sqlx::query("INSERT INTO control.environments (environment_id, project_id, name, cluster) VALUES ($1, $2, $3, $4)").bind(environment_id).bind(project_id).bind(&request.name).bind(&request.cluster).execute(pool).await {
        Ok(_) => (StatusCode::CREATED, Json(EnvironmentResponse { environment_id: environment_id.to_string(), project_id: project_id.to_string(), name: request.name, cluster: request.cluster })).into_response(),
        Err(_) => error(StatusCode::CONFLICT, "environment_conflict", "environment name already exists or project was not found"),
    }
}

/// Lists environments owned by the caller's project.
#[utoipa::path(get, path = "/v1/control/projects/{project_id}/environments", params(("project_id" = String, Path)), responses((status = 200, body = [EnvironmentResponse]), (status = 401, body = ApiError), (status = 403, body = ApiError)))]
pub async fn list_environments(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(project_id): Path<Uuid>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "environment listing requires durable storage",
        );
    };
    match sqlx::query("SELECT environment_id, project_id, name, cluster FROM control.environments WHERE project_id = $1 ORDER BY created_at, environment_id").bind(project_id).fetch_all(pool).await {
        Ok(rows) => (StatusCode::OK, Json(rows.into_iter().map(|row| EnvironmentResponse { environment_id: row.get::<Uuid, _>("environment_id").to_string(), project_id: row.get::<Uuid, _>("project_id").to_string(), name: row.get("name"), cluster: row.get("cluster") }).collect::<Vec<_>>())).into_response(),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "environment listing failed"),
    }
}

/// Registers an HTTPS RPC endpoint for an environment in the caller's project.
#[utoipa::path(post, path = "/v1/control/projects/{project_id}/environments/{environment_id}/routes", params(("project_id" = String, Path), ("environment_id" = String, Path)), request_body = CreateRouteRequest, responses((status = 201, body = RouteResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError)))]
pub async fn create_route(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path((project_id, environment_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<CreateRouteRequest>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    if !valid_text(&request.name, 120) || !valid_rpc_endpoint(&request.endpoint) {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_route_request",
            "route name or HTTPS endpoint is invalid",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "route provisioning requires durable storage",
        );
    };
    let route_id = Uuid::now_v7();
    match sqlx::query("INSERT INTO control.routes (route_id, environment_id, name, endpoint) SELECT $1, environment_id, $3, $4 FROM control.environments WHERE environment_id = $2 AND project_id = $5")
        .bind(route_id).bind(environment_id).bind(&request.name).bind(&request.endpoint).bind(project_id).execute(pool).await {
        Ok(result) if result.rows_affected() == 1 => (StatusCode::CREATED, Json(RouteResponse { route_id: route_id.to_string(), environment_id: environment_id.to_string(), name: request.name, enabled: true })).into_response(),
        Ok(_) => error(StatusCode::NOT_FOUND, "environment_not_found", "environment was not found in this project"),
        Err(_) => error(StatusCode::CONFLICT, "route_conflict", "route name already exists"),
    }
}

/// Lists safe route metadata for an environment. It never includes endpoint URLs.
#[utoipa::path(get, path = "/v1/control/projects/{project_id}/environments/{environment_id}/routes", params(("project_id" = String, Path), ("environment_id" = String, Path)), responses((status = 200, body = [RouteResponse]), (status = 401, body = ApiError), (status = 403, body = ApiError)))]
pub async fn list_routes(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path((project_id, environment_id)): Path<(Uuid, Uuid)>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "route listing requires durable storage",
        );
    };
    match sqlx::query("SELECT r.route_id, r.environment_id, r.name, r.enabled FROM control.routes r JOIN control.environments e ON e.environment_id = r.environment_id WHERE r.environment_id = $1 AND e.project_id = $2 ORDER BY r.created_at, r.route_id")
        .bind(environment_id).bind(project_id).fetch_all(pool).await {
        Ok(rows) => (StatusCode::OK, Json(rows.into_iter().map(|row| RouteResponse { route_id: row.get::<Uuid, _>("route_id").to_string(), environment_id: row.get::<Uuid, _>("environment_id").to_string(), name: row.get("name"), enabled: row.get("enabled") }).collect::<Vec<_>>())).into_response(),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "route listing failed"),
    }
}

/// Disables an RPC route so future observation jobs do not use it.
#[utoipa::path(post, path = "/v1/control/projects/{project_id}/environments/{environment_id}/routes/{route_id}/disable", params(("project_id" = String, Path), ("environment_id" = String, Path), ("route_id" = String, Path)), responses((status = 204), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError)))]
pub async fn disable_route(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path((project_id, environment_id, route_id)): Path<(Uuid, Uuid, Uuid)>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "route management requires durable storage",
        );
    };
    match sqlx::query("UPDATE control.routes r SET enabled = false FROM control.environments e WHERE r.route_id = $1 AND r.environment_id = $2 AND e.environment_id = r.environment_id AND e.project_id = $3")
        .bind(route_id).bind(environment_id).bind(project_id).execute(pool).await {
        Ok(result) if result.rows_affected() == 1 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => error(StatusCode::NOT_FOUND, "route_not_found", "route was not found"),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "route disable failed"),
    }
}

/// Mints a least-privilege token for the caller's project.
#[utoipa::path(post, path = "/v1/control/projects/{project_id}/tokens", params(("project_id" = String, Path)), request_body = CreateTokenRequest, responses((status = 201, body = CreatedTokenResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError)))]
pub async fn create_token(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(project_id): Path<Uuid>,
    Json(request): Json<CreateTokenRequest>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    if !valid_text(&request.name, 120) || !valid_scopes(&request.scopes) {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_token_request",
            "token name or scopes are invalid",
        );
    }
    let expires_at = match request.expires_at.as_deref() {
        Some(value) => match OffsetDateTime::parse(value, &Rfc3339) {
            Ok(value) if value > OffsetDateTime::now_utc() => Some(value),
            _ => {
                return error(
                    StatusCode::BAD_REQUEST,
                    "invalid_token_expiry",
                    "expires_at must be a future RFC 3339 timestamp",
                );
            }
        },
        None => None,
    };
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "token provisioning requires durable storage",
        );
    };
    let Ok((token, token_prefix)) = issue_token() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "token_generation_failed",
            "could not generate a secure API token",
        );
    };
    let token_id = Uuid::now_v7();
    match sqlx::query("INSERT INTO control.api_tokens (token_id, project_id, name, token_prefix, token_hash, scopes, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)").bind(token_id).bind(project_id).bind(&request.name).bind(&token_prefix).bind(auth::hash_token(&token).to_vec()).bind(&request.scopes).bind(expires_at).execute(pool).await {
        Ok(_) => (StatusCode::CREATED, Json(CreatedTokenResponse { token_id: token_id.to_string(), name: request.name, token_prefix, token, scopes: request.scopes, expires_at: format_time(expires_at) })).into_response(),
        Err(_) => error(StatusCode::CONFLICT, "token_conflict", "token name already exists or project was not found"),
    }
}

/// Lists token metadata without revealing hashes or plaintext credentials.
#[utoipa::path(get, path = "/v1/control/projects/{project_id}/tokens", params(("project_id" = String, Path)), responses((status = 200, body = [TokenResponse]), (status = 401, body = ApiError), (status = 403, body = ApiError)))]
pub async fn list_tokens(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(project_id): Path<Uuid>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "token listing requires durable storage",
        );
    };
    match sqlx::query("SELECT token_id, name, token_prefix, scopes, expires_at, revoked_at FROM control.api_tokens WHERE project_id = $1 ORDER BY created_at, token_id").bind(project_id).fetch_all(pool).await {
        Ok(rows) => (StatusCode::OK, Json(rows.into_iter().map(|row| TokenResponse { token_id: row.get::<Uuid, _>("token_id").to_string(), name: row.get("name"), token_prefix: row.get("token_prefix"), scopes: row.get("scopes"), expires_at: format_time(row.get("expires_at")), revoked_at: format_time(row.get("revoked_at")) }).collect::<Vec<_>>())).into_response(),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "token listing failed"),
    }
}

/// Revokes a token immediately. Revocation is idempotent for a token in this project.
#[utoipa::path(post, path = "/v1/control/projects/{project_id}/tokens/{token_id}/revoke", params(("project_id" = String, Path), ("token_id" = String, Path)), responses((status = 204), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError)))]
pub async fn revoke_token(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path((project_id, token_id)): Path<(Uuid, Uuid)>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "token revocation requires durable storage",
        );
    };
    match sqlx::query("UPDATE control.api_tokens SET revoked_at = COALESCE(revoked_at, now()) WHERE token_id = $1 AND project_id = $2").bind(token_id).bind(project_id).execute(pool).await {
        Ok(result) if result.rows_affected() == 1 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => error(StatusCode::NOT_FOUND, "token_not_found", "token was not found"),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "token revocation failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::{issue_token, valid_rpc_endpoint, valid_scopes};

    #[test]
    fn issued_tokens_are_prefixed_and_have_256_random_bits() {
        let (token, prefix) = issue_token().expect("system CSPRNG available");
        assert!(token.starts_with("lf_"));
        assert_eq!(token.len(), 67);
        assert_eq!(prefix, token[..11]);
    }

    #[test]
    fn token_scope_sets_must_be_known_and_unique() {
        assert!(valid_scopes(&["ingest:write".into()]));
        assert!(!valid_scopes(&[]));
        assert!(!valid_scopes(&["nope".into()]));
        assert!(!valid_scopes(&[
            "ingest:write".into(),
            "ingest:write".into()
        ]));
    }

    #[test]
    fn rpc_endpoint_must_be_https_and_never_embed_credentials() {
        assert!(valid_rpc_endpoint("https://api.mainnet-beta.solana.com"));
        assert!(!valid_rpc_endpoint("http://localhost:8899"));
        assert!(!valid_rpc_endpoint("https://key@rpc.example"));
    }
}
