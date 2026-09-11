//! Authenticated, redacted system-status read model.

use crate::auth::{ApiTokenRecord, AuthError, authorize};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::ToSchema;

/// Internal snapshot supplied by health probes and workers.
#[derive(Debug, Clone)]
pub struct SystemStatusSnapshot {
    pub database_ready: bool,
    pub queue_depth: u64,
    pub projection_lag_seconds: u64,
    pub observer_routes: u32,
    pub unhealthy_observer_routes: u32,
    pub schema_version: String,
    pub rule_set_version: String,
    pub retention_days: u32,
}

/// Safe component-level status exposed to an administrator.
#[derive(Debug, Clone, Serialize, ToSchema, PartialEq, Eq)]
pub struct DetailedSystemStatus {
    pub status: Status,
    pub database_ready: bool,
    pub queue_depth: u64,
    pub projection_lag_seconds: u64,
    pub observer_routes: u32,
    pub unhealthy_observer_routes: u32,
    pub schema_version: String,
    pub rule_set_version: String,
    pub retention_days: u32,
}

/// Overall status derived from dependency health, without exposing secrets.
#[derive(Debug, Clone, Copy, Serialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Degraded,
}

/// Authorizes and builds detailed status. Requires the `system:read` scope.
pub fn authenticated_status(
    authorization: Option<&str>,
    records: &[ApiTokenRecord],
    now: OffsetDateTime,
    snapshot: SystemStatusSnapshot,
) -> Result<DetailedSystemStatus, AuthError> {
    authorize(authorization, records, "system:read", now)?;
    let degraded = !snapshot.database_ready || snapshot.unhealthy_observer_routes > 0;
    Ok(DetailedSystemStatus {
        status: if degraded {
            Status::Degraded
        } else {
            Status::Ok
        },
        database_ready: snapshot.database_ready,
        queue_depth: snapshot.queue_depth,
        projection_lag_seconds: snapshot.projection_lag_seconds,
        observer_routes: snapshot.observer_routes,
        unhealthy_observer_routes: snapshot.unhealthy_observer_routes,
        schema_version: snapshot.schema_version,
        rule_set_version: snapshot.rule_set_version,
        retention_days: snapshot.retention_days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::hash_token;

    fn snapshot() -> SystemStatusSnapshot {
        SystemStatusSnapshot {
            database_ready: true,
            queue_depth: 4,
            projection_lag_seconds: 2,
            observer_routes: 2,
            unhealthy_observer_routes: 0,
            schema_version: "events-v1".into(),
            rule_set_version: "rules-v1".into(),
            retention_days: 30,
        }
    }

    #[test]
    fn requires_admin_scope_and_marks_degraded_dependencies() {
        let token = ApiTokenRecord {
            token_hash: hash_token("admin"),
            scopes: vec!["system:read".into()],
            expires_at: None,
            revoked_at: None,
        };
        assert_eq!(
            authenticated_status(
                None,
                &[token.clone()],
                OffsetDateTime::UNIX_EPOCH,
                snapshot()
            ),
            Err(AuthError::MissingBearer)
        );
        let mut degraded = snapshot();
        degraded.unhealthy_observer_routes = 1;
        let result = authenticated_status(
            Some("Bearer admin"),
            &[token],
            OffsetDateTime::UNIX_EPOCH,
            degraded,
        )
        .unwrap();
        assert_eq!(result.status, Status::Degraded);
        assert_eq!(result.queue_depth, 4);
    }
}
