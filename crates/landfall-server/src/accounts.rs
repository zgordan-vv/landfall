//! Interactive SaaS identity, workspace membership, invitations, and audit APIs.

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use getrandom::fill;
use password_hash::{SaltString, rand_core::OsRng};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{PgPool, Row};
use time::{Duration, OffsetDateTime};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, AppState, auth};

const SESSION_DAYS: i64 = 14;
const INVITATION_DAYS: i64 = 7;

/// Interactive user identity added to SaaS-only request handlers.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub display_name: String,
    pub password: String,
    pub workspace_name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateInvitationRequest {
    pub email: String,
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AcceptInvitationRequest {
    pub token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMemberRoleRequest {
    pub role: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkspaceProjectRequest {
    pub name: String,
    pub initial_token_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceResponse {
    pub workspace_id: String,
    pub name: String,
    pub slug: String,
    pub role: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub expires_at: String,
    pub workspace: WorkspaceResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MemberResponse {
    pub user_id: String,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct InvitationResponse {
    pub invitation_id: String,
    pub email: String,
    pub role: String,
    pub expires_at: String,
    /// Returned only at invitation creation so an email service may deliver it.
    pub invitation_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct WorkspaceProjectResponse {
    pub project_id: String,
    pub name: String,
    pub initial_token: String,
    pub token_prefix: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditRecordResponse {
    pub audit_id: String,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub created_at: String,
}

pub async fn authenticate_user(
    State(state): State<std::sync::Arc<AppState>>,
    mut request: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("interactive authentication requires durable storage");
    };
    let Some(token) = bearer(&request) else {
        return unauthorized();
    };
    let token_hash = auth::hash_token(token);
    let user_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT s.user_id FROM identity.sessions s JOIN identity.users u ON u.user_id = s.user_id WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND s.expires_at > now() AND u.disabled_at IS NULL",
    )
    .bind(token_hash.to_vec())
    .fetch_optional(pool)
    .await;
    match user_id {
        Ok(Some(user_id)) => {
            let _ = sqlx::query(
                "UPDATE identity.sessions SET last_seen_at = now() WHERE token_hash = $1",
            )
            .bind(token_hash.to_vec())
            .execute(pool)
            .await;
            request
                .extensions_mut()
                .insert(AuthenticatedUser { user_id });
            next.run(request).await
        }
        _ => unauthorized(),
    }
}

pub async fn register(
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> axum::response::Response {
    if !valid_email(&request.email)
        || !valid_text(&request.display_name, 120)
        || !valid_text(&request.workspace_name, 120)
        || !valid_password(&request.password)
    {
        return bad_request(
            "registration requires a valid email, name, workspace name, and a password of at least 12 characters",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("registration requires durable storage");
    };
    let Ok(password_hash) = hash_password(&request.password) else {
        return unavailable("password hashing failed");
    };
    let user_id = Uuid::now_v7();
    let workspace_id = Uuid::now_v7();
    let slug = unique_slug(&request.workspace_name, workspace_id);
    let Ok((token, token_hash)) = issue_secret("lfs_") else {
        return unavailable("session generation failed");
    };
    let expires_at = OffsetDateTime::now_utc() + Duration::days(SESSION_DAYS);
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable("registration failed"),
    };
    let created = async {
        sqlx::query("INSERT INTO identity.users (user_id, email, display_name, email_verified_at) VALUES ($1, $2, $3, now())")
            .bind(user_id).bind(normalize_email(&request.email)).bind(&request.display_name).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO identity.password_credentials (user_id, password_hash) VALUES ($1, $2)")
            .bind(user_id).bind(password_hash).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO control.workspaces (workspace_id, name, slug) VALUES ($1, $2, $3)")
            .bind(workspace_id).bind(&request.workspace_name).bind(&slug).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO control.workspace_members (workspace_id, user_id, role) VALUES ($1, $2, 'owner')")
            .bind(workspace_id).bind(user_id).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO identity.sessions (session_id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)")
            .bind(Uuid::now_v7()).bind(user_id).bind(token_hash).bind(expires_at).execute(&mut *tx).await?;
        audit(&mut tx, Some(workspace_id), None, Some(user_id), "account.registered", "user", Some(user_id)).await;
        Ok::<(), sqlx::Error>(())
    }.await;
    if created.is_err() || tx.commit().await.is_err() {
        return conflict("an account with this email or workspace slug already exists");
    }
    (
        StatusCode::CREATED,
        Json(SessionResponse {
            user: user_response(user_id, &request.email, &request.display_name),
            access_token: token,
            expires_at: expires_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            workspace: workspace_response(workspace_id, &request.workspace_name, slug, "owner"),
        }),
    )
        .into_response()
}

pub async fn login(
    State(state): State<std::sync::Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("login requires durable storage");
    };
    let row = sqlx::query("SELECT u.user_id, u.email, u.display_name, c.password_hash FROM identity.users u JOIN identity.password_credentials c ON c.user_id = u.user_id WHERE lower(u.email) = lower($1) AND u.disabled_at IS NULL")
        .bind(normalize_email(&request.email)).fetch_optional(pool).await;
    let Ok(Some(row)) = row else {
        return unauthorized();
    };
    let password_hash: String = row.get("password_hash");
    if !verify_password(&request.password, &password_hash) {
        return unauthorized();
    }
    let user_id: Uuid = row.get("user_id");
    let workspace = match first_workspace(pool, user_id).await {
        Ok(Some(value)) => value,
        _ => return unavailable("account has no accessible workspace"),
    };
    let Ok((token, token_hash)) = issue_secret("lfs_") else {
        return unavailable("session generation failed");
    };
    let expires_at = OffsetDateTime::now_utc() + Duration::days(SESSION_DAYS);
    if sqlx::query("INSERT INTO identity.sessions (session_id, user_id, token_hash, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(Uuid::now_v7()).bind(user_id).bind(token_hash).bind(expires_at).execute(pool).await.is_err() { return unavailable("login failed"); }
    let _ = write_audit(
        pool,
        Some(workspace.0),
        None,
        Some(user_id),
        "session.created",
        "session",
        None,
    )
    .await;
    (
        StatusCode::OK,
        Json(SessionResponse {
            user: user_response(
                user_id,
                &row.get::<String, _>("email"),
                &row.get::<String, _>("display_name"),
            ),
            access_token: token,
            expires_at: expires_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            workspace: workspace_response(workspace.0, &workspace.1, workspace.2, &workspace.3),
        }),
    )
        .into_response()
}

pub async fn logout(
    State(state): State<std::sync::Arc<AppState>>,
    headers: HeaderMap,
    Extension(user): Extension<AuthenticatedUser>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("logout requires durable storage");
    };
    let Some(token) = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return unauthorized();
    };
    match sqlx::query("UPDATE identity.sessions SET revoked_at = now() WHERE user_id = $1 AND token_hash = $2 AND revoked_at IS NULL")
        .bind(user.user_id).bind(auth::hash_token(token).to_vec()).execute(pool).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(), Err(_) => unavailable("logout failed"),
    }
}

pub async fn me(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("account lookup requires durable storage");
    };
    match sqlx::query("SELECT user_id, email, display_name FROM identity.users WHERE user_id = $1")
        .bind(user.user_id)
        .fetch_optional(pool)
        .await
    {
        Ok(Some(row)) => Json(user_response(
            row.get("user_id"),
            &row.get::<String, _>("email"),
            &row.get::<String, _>("display_name"),
        ))
        .into_response(),
        _ => unauthorized(),
    }
}

pub async fn list_workspaces(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("workspace listing requires durable storage");
    };
    match sqlx::query("SELECT w.workspace_id, w.name, w.slug, m.role FROM control.workspaces w JOIN control.workspace_members m ON m.workspace_id = w.workspace_id WHERE m.user_id = $1 ORDER BY w.created_at")
        .bind(user.user_id).fetch_all(pool).await {
        Ok(rows) => Json(rows.into_iter().map(|row| workspace_response(row.get("workspace_id"), &row.get::<String, _>("name"), row.get("slug"), &row.get::<String, _>("role"))).collect::<Vec<_>>()).into_response(),
        Err(_) => unavailable("workspace listing failed"),
    }
}

pub async fn create_workspace(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<CreateWorkspaceRequest>,
) -> axum::response::Response {
    if !valid_text(&request.name, 120) {
        return bad_request("workspace name is required");
    }
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("workspace creation requires durable storage");
    };
    let workspace_id = Uuid::now_v7();
    let slug = unique_slug(&request.name, workspace_id);
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable("workspace creation failed"),
    };
    let created = async { sqlx::query("INSERT INTO control.workspaces (workspace_id, name, slug) VALUES ($1, $2, $3)").bind(workspace_id).bind(&request.name).bind(&slug).execute(&mut *tx).await?; sqlx::query("INSERT INTO control.workspace_members (workspace_id, user_id, role) VALUES ($1, $2, 'owner')").bind(workspace_id).bind(user.user_id).execute(&mut *tx).await?; audit(&mut tx, Some(workspace_id), None, Some(user.user_id), "workspace.created", "workspace", Some(workspace_id)).await; Ok::<(), sqlx::Error>(()) }.await;
    if created.is_err() || tx.commit().await.is_err() {
        return conflict("workspace could not be created");
    }
    (
        StatusCode::CREATED,
        Json(workspace_response(
            workspace_id,
            &request.name,
            slug,
            "owner",
        )),
    )
        .into_response()
}

pub async fn list_members(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(workspace_id): Path<Uuid>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("member listing requires durable storage");
    };
    if !has_role(
        pool,
        user.user_id,
        workspace_id,
        &["owner", "admin", "developer", "viewer"],
    )
    .await
    {
        return forbidden();
    }
    match sqlx::query("SELECT u.user_id, u.email, u.display_name, m.role, m.joined_at FROM control.workspace_members m JOIN identity.users u ON u.user_id = m.user_id WHERE m.workspace_id = $1 ORDER BY m.joined_at, u.user_id").bind(workspace_id).fetch_all(pool).await {
        Ok(rows) => Json(rows.into_iter().map(|row| MemberResponse { user_id: row.get::<Uuid, _>("user_id").to_string(), email: row.get("email"), display_name: row.get("display_name"), role: row.get("role"), joined_at: row.get::<OffsetDateTime, _>("joined_at").format(&time::format_description::well_known::Rfc3339).unwrap_or_default() }).collect::<Vec<_>>()).into_response(),
        Err(_) => unavailable("member listing failed"),
    }
}

pub async fn invite_member(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateInvitationRequest>,
) -> axum::response::Response {
    if !valid_email(&request.email)
        || !matches!(request.role.as_str(), "admin" | "developer" | "viewer")
    {
        return bad_request("invitation requires an email and a non-owner role");
    }
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("invitation requires durable storage");
    };
    if !has_role(pool, user.user_id, workspace_id, &["owner", "admin"]).await {
        return forbidden();
    }
    let Ok((token, token_hash)) = issue_secret("lfi_") else {
        return unavailable("invitation generation failed");
    };
    let invitation_id = Uuid::now_v7();
    let expires_at = OffsetDateTime::now_utc() + Duration::days(INVITATION_DAYS);
    match sqlx::query("INSERT INTO control.workspace_invitations (invitation_id, workspace_id, email, role, token_hash, expires_at, created_by) VALUES ($1, $2, $3, $4, $5, $6, $7)").bind(invitation_id).bind(workspace_id).bind(normalize_email(&request.email)).bind(&request.role).bind(token_hash).bind(expires_at).bind(user.user_id).execute(pool).await {
        Ok(_) => { let _ = write_audit(pool, Some(workspace_id), None, Some(user.user_id), "workspace.invitation_created", "invitation", Some(invitation_id)).await; (StatusCode::CREATED, Json(InvitationResponse { invitation_id: invitation_id.to_string(), email: normalize_email(&request.email), role: request.role, expires_at: expires_at.format(&time::format_description::well_known::Rfc3339).unwrap_or_default(), invitation_token: token })).into_response() },
        Err(_) => conflict("an active invitation already exists or the workspace was not found"),
    }
}

pub async fn accept_invitation(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(request): Json<AcceptInvitationRequest>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("invitation acceptance requires durable storage");
    };
    let token_hash = auth::hash_token(&request.token);
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable("invitation acceptance failed"),
    };
    let invitation = sqlx::query("SELECT i.invitation_id, i.workspace_id, i.email, i.role FROM control.workspace_invitations i JOIN identity.users u ON u.user_id = $1 WHERE i.token_hash = $2 AND i.accepted_at IS NULL AND i.revoked_at IS NULL AND i.expires_at > now() AND lower(i.email) = lower(u.email) FOR UPDATE")
        .bind(user.user_id).bind(token_hash.to_vec()).fetch_optional(&mut *tx).await;
    let Ok(Some(row)) = invitation else {
        return bad_request("invitation is invalid, expired, or addressed to another account");
    };
    let invitation_id: Uuid = row.get("invitation_id");
    let workspace_id: Uuid = row.get("workspace_id");
    let role: String = row.get("role");
    let accepted = async { sqlx::query("INSERT INTO control.workspace_members (workspace_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT (workspace_id, user_id) DO UPDATE SET role = EXCLUDED.role").bind(workspace_id).bind(user.user_id).bind(&role).execute(&mut *tx).await?; sqlx::query("UPDATE control.workspace_invitations SET accepted_at = now() WHERE invitation_id = $1").bind(invitation_id).execute(&mut *tx).await?; audit(&mut tx, Some(workspace_id), None, Some(user.user_id), "workspace.invitation_accepted", "invitation", Some(invitation_id)).await; Ok::<(), sqlx::Error>(()) }.await;
    if accepted.is_err() || tx.commit().await.is_err() {
        return unavailable("invitation acceptance failed");
    }
    StatusCode::NO_CONTENT.into_response()
}

pub async fn update_member_role(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path((workspace_id, member_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpdateMemberRoleRequest>,
) -> axum::response::Response {
    if !matches!(request.role.as_str(), "admin" | "developer" | "viewer") {
        return bad_request("only admin, developer, and viewer roles may be assigned");
    }
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("role update requires durable storage");
    };
    if !has_role(pool, user.user_id, workspace_id, &["owner"]).await {
        return forbidden();
    }
    match sqlx::query("UPDATE control.workspace_members SET role = $1 WHERE workspace_id = $2 AND user_id = $3 AND role <> 'owner'").bind(&request.role).bind(workspace_id).bind(member_id).execute(pool).await {
        Ok(result) if result.rows_affected() == 1 => { let _ = write_audit(pool, Some(workspace_id), None, Some(user.user_id), "workspace.member_role_updated", "user", Some(member_id)).await; StatusCode::NO_CONTENT.into_response() },
        Ok(_) => bad_request("member was not found or is a workspace owner"), Err(_) => unavailable("role update failed"),
    }
}

pub async fn create_workspace_project(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(workspace_id): Path<Uuid>,
    Json(request): Json<CreateWorkspaceProjectRequest>,
) -> axum::response::Response {
    if !valid_text(&request.name, 200) || !valid_text(&request.initial_token_name, 120) {
        return bad_request("project and token names are required");
    }
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("project creation requires durable storage");
    };
    if !has_role(
        pool,
        user.user_id,
        workspace_id,
        &["owner", "admin", "developer"],
    )
    .await
    {
        return forbidden();
    }
    let Ok((token, token_hash)) = issue_secret("lf_") else {
        return unavailable("token generation failed");
    };
    let project_id = Uuid::now_v7();
    let token_id = Uuid::now_v7();
    let prefix = token[..11].to_owned();
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return unavailable("project creation failed"),
    };
    let created = async { sqlx::query("INSERT INTO control.projects (project_id, workspace_id, name) VALUES ($1, $2, $3)").bind(project_id).bind(workspace_id).bind(&request.name).execute(&mut *tx).await?; sqlx::query("INSERT INTO control.api_tokens (token_id, project_id, name, token_prefix, token_hash, scopes) VALUES ($1, $2, $3, $4, $5, $6)").bind(token_id).bind(project_id).bind(&request.initial_token_name).bind(&prefix).bind(token_hash).bind(vec!["project:admin"]).execute(&mut *tx).await?; audit(&mut tx, Some(workspace_id), Some(project_id), Some(user.user_id), "project.created", "project", Some(project_id)).await; Ok::<(), sqlx::Error>(()) }.await;
    if created.is_err() || tx.commit().await.is_err() {
        return conflict("project could not be created");
    }
    (
        StatusCode::CREATED,
        Json(WorkspaceProjectResponse {
            project_id: project_id.to_string(),
            name: request.name,
            initial_token: token,
            token_prefix: prefix,
        }),
    )
        .into_response()
}

pub async fn list_workspace_projects(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(workspace_id): Path<Uuid>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("project listing requires durable storage");
    };
    if !has_role(
        pool,
        user.user_id,
        workspace_id,
        &["owner", "admin", "developer", "viewer"],
    )
    .await
    {
        return forbidden();
    }
    match sqlx::query("SELECT project_id, name FROM control.projects WHERE workspace_id = $1 ORDER BY created_at, project_id").bind(workspace_id).fetch_all(pool).await {
        Ok(rows) => Json(rows.into_iter().map(|row| json!({"project_id": row.get::<Uuid, _>("project_id").to_string(), "name": row.get::<String, _>("name")})).collect::<Vec<_>>()).into_response(),
        Err(_) => unavailable("project listing failed"),
    }
}

pub async fn list_audit_log(
    State(state): State<std::sync::Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(workspace_id): Path<Uuid>,
) -> axum::response::Response {
    let Some(pool) = state.pool.as_ref() else {
        return unavailable("audit log requires durable storage");
    };
    if !has_role(pool, user.user_id, workspace_id, &["owner", "admin"]).await {
        return forbidden();
    }
    match sqlx::query("SELECT audit_id, action, target_type, target_id, created_at FROM control.audit_log WHERE workspace_id = $1 ORDER BY created_at DESC, audit_id DESC LIMIT 100").bind(workspace_id).fetch_all(pool).await {
        Ok(rows) => Json(rows.into_iter().map(|row| AuditRecordResponse { audit_id: row.get::<Uuid, _>("audit_id").to_string(), action: row.get("action"), target_type: row.get("target_type"), target_id: row.get::<Option<Uuid>, _>("target_id").map(|id| id.to_string()), created_at: row.get::<OffsetDateTime, _>("created_at").format(&time::format_description::well_known::Rfc3339).unwrap_or_default() }).collect::<Vec<_>>()).into_response(),
        Err(_) => unavailable("audit log listing failed"),
    }
}

fn bearer(request: &axum::http::Request<axum::body::Body>) -> Option<&str> {
    request
        .headers()
        .get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}
fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max
}
fn valid_email(value: &str) -> bool {
    value.len() <= 320
        && value.contains('@')
        && value
            .rsplit_once('@')
            .is_some_and(|(_, domain)| domain.contains('.'))
}
fn normalize_email(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
fn valid_password(value: &str) -> bool {
    value.len() >= 12 && value.len() <= 256
}
fn unique_slug(name: &str, id: Uuid) -> String {
    let base: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let base = base.trim_matches('-');
    format!(
        "{}-{}",
        if base.is_empty() {
            "workspace"
        } else {
            &base[..base.len().min(65)]
        },
        &id.to_string()[..8]
    )
}
fn hash_password(password: &str) -> Result<String, ()> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ())
}
fn verify_password(password: &str, stored: &str) -> bool {
    PasswordHash::new(stored).ok().is_some_and(|hash| {
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
}
fn issue_secret(prefix: &str) -> Result<(String, Vec<u8>), ()> {
    let mut bytes = [0_u8; 32];
    fill(&mut bytes).map_err(|_| ())?;
    let mut token = String::with_capacity(prefix.len() + 64);
    token.push_str(prefix);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(token, "{byte:02x}");
    }
    let hash = auth::hash_token(&token).to_vec();
    Ok((token, hash))
}
fn user_response(user_id: Uuid, email: &str, display_name: &str) -> UserResponse {
    UserResponse {
        user_id: user_id.to_string(),
        email: email.to_owned(),
        display_name: display_name.to_owned(),
    }
}
fn workspace_response(
    workspace_id: Uuid,
    name: &str,
    slug: String,
    role: &str,
) -> WorkspaceResponse {
    WorkspaceResponse {
        workspace_id: workspace_id.to_string(),
        name: name.to_owned(),
        slug,
        role: role.to_owned(),
    }
}
async fn first_workspace(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Option<(Uuid, String, String, String)>, sqlx::Error> {
    sqlx::query("SELECT w.workspace_id, w.name, w.slug, m.role FROM control.workspaces w JOIN control.workspace_members m ON m.workspace_id = w.workspace_id WHERE m.user_id = $1 ORDER BY w.created_at LIMIT 1").bind(user_id).fetch_optional(pool).await.map(|row| row.map(|value| (value.get("workspace_id"), value.get("name"), value.get("slug"), value.get("role"))))
}
async fn has_role(pool: &PgPool, user_id: Uuid, workspace_id: Uuid, roles: &[&str]) -> bool {
    sqlx::query_scalar::<_, String>(
        "SELECT role FROM control.workspace_members WHERE workspace_id = $1 AND user_id = $2",
    )
    .bind(workspace_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .is_some_and(|role| roles.contains(&role.as_str()))
}
async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: Option<Uuid>,
    project_id: Option<Uuid>,
    actor_user_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
) {
    let _ = sqlx::query("INSERT INTO control.audit_log (audit_id, workspace_id, project_id, actor_user_id, action, target_type, target_id) VALUES ($1, $2, $3, $4, $5, $6, $7)").bind(Uuid::now_v7()).bind(workspace_id).bind(project_id).bind(actor_user_id).bind(action).bind(target_type).bind(target_id).execute(&mut **tx).await;
}
async fn write_audit(
    pool: &PgPool,
    workspace_id: Option<Uuid>,
    project_id: Option<Uuid>,
    actor_user_id: Option<Uuid>,
    action: &str,
    target_type: &str,
    target_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO control.audit_log (audit_id, workspace_id, project_id, actor_user_id, action, target_type, target_id) VALUES ($1, $2, $3, $4, $5, $6, $7)").bind(Uuid::now_v7()).bind(workspace_id).bind(project_id).bind(actor_user_id).bind(action).bind(target_type).bind(target_id).execute(pool).await.map(|_| ())
}
fn bad_request(message: &'static str) -> axum::response::Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiError {
            code: "invalid_account_input",
            message,
        }),
    )
        .into_response()
}
fn unauthorized() -> axum::response::Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiError {
            code: "unauthorized",
            message: "valid account authentication is required",
        }),
    )
        .into_response()
}
fn forbidden() -> axum::response::Response {
    (
        StatusCode::FORBIDDEN,
        Json(ApiError {
            code: "workspace_access_denied",
            message: "workspace role does not grant this operation",
        }),
    )
        .into_response()
}
fn conflict(message: &'static str) -> axum::response::Response {
    (
        StatusCode::CONFLICT,
        Json(ApiError {
            code: "account_conflict",
            message,
        }),
    )
        .into_response()
}
fn unavailable(message: &'static str) -> axum::response::Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(ApiError {
            code: "storage_unavailable",
            message,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{hash_password, unique_slug, valid_password, verify_password};
    use uuid::Uuid;
    #[test]
    fn password_hash_is_not_reversible_and_verifies() {
        let hash = hash_password("correct horse battery staple").expect("hash");
        assert_ne!(hash, "correct horse battery staple");
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("other password", &hash));
    }
    #[test]
    fn password_policy_and_slug_are_deterministic() {
        assert!(!valid_password("short"));
        assert!(valid_password("twelve chars!"));
        assert_eq!(
            unique_slug("Acme Payments", Uuid::nil()),
            "acme-payments-00000000"
        );
    }
}
