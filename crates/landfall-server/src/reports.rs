//! Durable PostgreSQL-backed report export endpoints.

use axum::{
    Json,
    extract::{Extension, Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use time::format_description::well_known::Rfc3339;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, AppState, AuthenticatedToken};

const MAX_REPORT_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateReportRequest {
    pub title: String,
    pub privacy_profile: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReportResponse {
    pub report_id: String,
    pub title: String,
    pub status: String,
    pub privacy_profile: String,
    pub trace_count: i64,
    pub created_at: String,
}

fn error(
    status: StatusCode,
    code: &'static str,
    message: &'static str,
) -> axum::response::Response {
    (status, Json(ApiError { code, message })).into_response()
}

fn project_admin(principal: Option<Extension<AuthenticatedToken>>, project_id: Uuid) -> bool {
    principal.is_some_and(|Extension(principal)| {
        principal.project_id == project_id
            && principal
                .scopes
                .iter()
                .any(|scope| scope == "project:admin")
    })
}

fn profile_is_valid(profile: &str) -> bool {
    matches!(profile, "internal" | "shareable")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn render_html(title: &str, traces: &[serde_json::Value]) -> String {
    let rows = traces
        .iter()
        .map(|trace| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(trace["trace_id"].as_str().unwrap_or("[redacted]")),
                escape_html(trace["lifecycle_state"].as_str().unwrap_or("unknown")),
                escape_html(trace["landing_state"].as_str().unwrap_or("unknown")),
                escape_html(trace["execution_state"].as_str().unwrap_or("unknown")),
                escape_html(trace["updated_at"].as_str().unwrap_or("")),
            )
        })
        .collect::<String>();
    format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>{}</title><style>body{{font:16px system-ui,sans-serif;margin:2rem;max-width:1100px}}table{{border-collapse:collapse;width:100%}}th,td{{border:1px solid #ccd3dd;padding:.55rem;text-align:left}}th{{background:#f4f6f8}}</style><h1>{}</h1><p>Durable Landfall lifecycle export.</p><table><thead><tr><th>Trace</th><th>Lifecycle</th><th>Landing</th><th>Execution</th><th>Updated</th></tr></thead><tbody>{}</tbody></table></html>",
        escape_html(title),
        escape_html(title),
        rows
    )
}

/// Creates and persists a complete report export in PostgreSQL.
#[utoipa::path(post, path = "/v1/control/projects/{project_id}/reports", params(("project_id" = String, Path)), request_body = CreateReportRequest, responses((status = 201, body = ReportResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 503, body = ApiError)))]
pub async fn create_report(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path(project_id): Path<Uuid>,
    Json(request): Json<CreateReportRequest>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    if request.title.trim().is_empty()
        || request.title.chars().count() > 240
        || !profile_is_valid(&request.privacy_profile)
    {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_report_request",
            "title or privacy_profile is invalid",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "report export requires durable storage",
        );
    };
    let rows = match sqlx::query("SELECT trace_id, lifecycle_state, landing_state, execution_state, updated_at FROM reporting.traces WHERE project_id = $1 ORDER BY updated_at DESC, trace_id DESC LIMIT 10000")
        .bind(project_id).fetch_all(pool).await {
            Ok(value) => value,
            Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "report source query failed"),
        };
    let traces = rows
        .into_iter()
        .map(|row| {
            let trace_id = if request.privacy_profile == "shareable" {
                "[redacted]".to_owned()
            } else {
                row.get::<Uuid, _>("trace_id").to_string()
            };
            serde_json::json!({
                "trace_id": trace_id,
                "lifecycle_state": row.get::<String, _>("lifecycle_state"),
                "landing_state": row.get::<String, _>("landing_state"),
                "execution_state": row.get::<String, _>("execution_state"),
                "updated_at": row.get::<time::OffsetDateTime, _>("updated_at").format(&Rfc3339).unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    let created_at = time::OffsetDateTime::now_utc();
    let json = match serde_json::to_vec_pretty(&serde_json::json!({
        "title": request.title,
        "privacy_profile": request.privacy_profile,
        "generated_at": created_at.format(&Rfc3339).unwrap_or_default(),
        "traces": traces,
    })) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "report_render_failed",
                "report could not be rendered",
            );
        }
    };
    let document: serde_json::Value = match serde_json::from_slice(&json) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "report_render_failed",
                "report could not be rendered",
            );
        }
    };
    let html = render_html(
        &request.title,
        document["traces"].as_array().unwrap_or(&Vec::new()),
    )
    .into_bytes();
    if json.len() > MAX_REPORT_BYTES || html.len() > MAX_REPORT_BYTES {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "report_too_large",
            "report exceeds the 10 MiB export limit; narrow the project data before exporting",
        );
    }
    let report_id = Uuid::now_v7();
    let mut transaction = match pool.begin().await {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "storage_unavailable",
                "report export could not start",
            );
        }
    };
    let write = async {
        sqlx::query("INSERT INTO reporting.report_runs (report_id, created_at, status) VALUES ($1,$2,'completed')")
            .bind(report_id).bind(created_at).execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO reporting.report_metadata (report_id, project_id, title, semantics_version, trace_count, created_at) VALUES ($1,$2,$3,$4,$5,$6)")
            .bind(report_id).bind(project_id).bind(&request.title).bind("live-trace-export-v1").bind(i32::try_from(document["traces"].as_array().map_or(0, Vec::len)).unwrap_or(i32::MAX)).bind(created_at).execute(&mut *transaction).await?;
        for (format, content) in [("json", &json), ("html", &html)] {
            sqlx::query("INSERT INTO reporting.report_artifacts (artifact_id, report_id, format, privacy_profile, storage_key, content_sha256, content_bytes, content, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                .bind(Uuid::now_v7()).bind(report_id).bind(format).bind(&request.privacy_profile).bind(format!("reports/{report_id}.{format}")).bind(Sha256::digest(content).to_vec()).bind(i64::try_from(content.len()).unwrap_or(i64::MAX)).bind(content).bind(created_at).execute(&mut *transaction).await?;
        }
        Result::<(), sqlx::Error>::Ok(())
    }.await;
    if write.is_err() || transaction.commit().await.is_err() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "report export could not be stored",
        );
    }
    (
        StatusCode::CREATED,
        Json(ReportResponse {
            report_id: report_id.to_string(),
            title: request.title,
            status: "completed".into(),
            privacy_profile: request.privacy_profile,
            trace_count: document["traces"]
                .as_array()
                .map_or(0, |items| items.len() as i64),
            created_at: created_at.format(&Rfc3339).unwrap_or_default(),
        }),
    )
        .into_response()
}

/// Lists durable report exports for one project.
#[utoipa::path(get, path = "/v1/control/projects/{project_id}/reports", params(("project_id" = String, Path)), responses((status = 200, body = [ReportResponse]), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 503, body = ApiError)))]
pub async fn list_reports(
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
            "report listing requires durable storage",
        );
    };
    match sqlx::query("SELECT r.report_id, r.status, m.title, m.trace_count, m.created_at, (SELECT privacy_profile FROM reporting.report_artifacts a WHERE a.report_id = r.report_id ORDER BY a.created_at LIMIT 1) AS privacy_profile FROM reporting.report_runs r JOIN reporting.report_metadata m ON m.report_id = r.report_id WHERE m.project_id = $1 ORDER BY m.created_at DESC, r.report_id DESC").bind(project_id).fetch_all(pool).await {
        Ok(rows) => (StatusCode::OK, Json(rows.into_iter().map(|row| ReportResponse { report_id: row.get::<Uuid, _>("report_id").to_string(), title: row.get("title"), status: row.get("status"), privacy_profile: row.get("privacy_profile"), trace_count: i64::from(row.get::<i32, _>("trace_count")), created_at: row.get::<time::OffsetDateTime, _>("created_at").format(&Rfc3339).unwrap_or_default() }).collect::<Vec<_>>())).into_response(),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "report listing failed"),
    }
}

/// Downloads the immutable JSON or HTML bytes of a durable report.
#[utoipa::path(get, path = "/v1/control/projects/{project_id}/reports/{report_id}/{format}", params(("project_id" = String, Path), ("report_id" = String, Path), ("format" = String, Path)), responses((status = 200), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError)))]
pub async fn download_report(
    State(state): State<std::sync::Arc<AppState>>,
    principal: Option<Extension<AuthenticatedToken>>,
    Path((project_id, report_id, format)): Path<(Uuid, Uuid, String)>,
) -> axum::response::Response {
    if !project_admin(principal, project_id) {
        return error(
            StatusCode::FORBIDDEN,
            "project_access_denied",
            "project administrator access is required",
        );
    }
    if !matches!(format.as_str(), "json" | "html") {
        return error(
            StatusCode::BAD_REQUEST,
            "invalid_report_format",
            "format must be json or html",
        );
    }
    let Some(pool) = state.pool.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "storage_unavailable",
            "report download requires durable storage",
        );
    };
    match sqlx::query("SELECT a.content FROM reporting.report_artifacts a JOIN reporting.report_metadata m ON m.report_id = a.report_id WHERE a.report_id = $1 AND m.project_id = $2 AND a.format = $3").bind(report_id).bind(project_id).bind(&format).fetch_optional(pool).await {
        Ok(Some(row)) => match row.get::<Option<Vec<u8>>, _>("content") {
            Some(content) => (
                [
                    (header::CONTENT_TYPE, if format == "json" { "application/json" } else { "text/html; charset=utf-8" }),
                    (header::CONTENT_DISPOSITION, if format == "json" { "attachment; filename=landfall-report.json" } else { "attachment; filename=landfall-report.html" }),
                ],
                content,
            ).into_response(),
            None => error(
                StatusCode::GONE,
                "report_artifact_unavailable",
                "this legacy report has metadata only; create a new durable export",
            ),
        },
        Ok(None) => error(StatusCode::NOT_FOUND, "report_not_found", "report artifact was not found"),
        Err(_) => error(StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable", "report download failed"),
    }
}
