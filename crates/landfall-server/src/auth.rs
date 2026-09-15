//! Hash-only bearer-token authentication policy.

use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ApiTokenRecord {
    pub token_id: Uuid,
    pub project_id: Uuid,
    pub token_hash: [u8; 32],
    pub scopes: Vec<String>,
    pub expires_at: Option<OffsetDateTime>,
    pub revoked_at: Option<OffsetDateTime>,
}

/// Authenticated identity attached to every production API request.
#[derive(Debug, Clone)]
pub struct AuthenticatedToken {
    pub project_id: Uuid,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AuthError {
    MissingBearer,
    InvalidBearer,
    UnknownToken,
    RevokedToken,
    ExpiredToken,
    MissingScope,
}

#[must_use]
pub fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

pub fn authorize<'a>(
    authorization: Option<&str>,
    records: &'a [ApiTokenRecord],
    required_scope: &str,
    now: OffsetDateTime,
) -> Result<&'a ApiTokenRecord, AuthError> {
    let header = authorization.ok_or(AuthError::MissingBearer)?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidBearer)?;
    if token.is_empty() || token.contains(char::is_whitespace) {
        return Err(AuthError::InvalidBearer);
    }
    let hash = hash_token(token);
    let record = records
        .iter()
        .find(|record| constant_time_eq(&record.token_hash, &hash))
        .ok_or(AuthError::UnknownToken)?;
    if record.revoked_at.is_some() {
        return Err(AuthError::RevokedToken);
    }
    if record.expires_at.is_some_and(|expiry| expiry <= now) {
        return Err(AuthError::ExpiredToken);
    }
    if !record.scopes.iter().any(|scope| scope == required_scope) {
        return Err(AuthError::MissingScope);
    }
    Ok(record)
}

/// Loads hash-only credentials and verifies a request without exposing token
/// existence through the public HTTP response.
pub async fn authenticate_database(
    pool: &PgPool,
    authorization: Option<&str>,
    required_scope: &str,
) -> Result<AuthenticatedToken, AuthError> {
    let rows = sqlx::query(
        "SELECT token_id, project_id, token_hash, scopes, expires_at, revoked_at FROM control.api_tokens",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| AuthError::UnknownToken)?;
    let records = rows
        .into_iter()
        .filter_map(|row| {
            let bytes: Vec<u8> = row.get("token_hash");
            let token_hash: [u8; 32] = bytes.try_into().ok()?;
            Some(ApiTokenRecord {
                token_id: row.get("token_id"),
                project_id: row.get("project_id"),
                token_hash,
                scopes: row.get("scopes"),
                expires_at: row.get("expires_at"),
                revoked_at: row.get("revoked_at"),
            })
        })
        .collect::<Vec<_>>();
    let record = authorize(
        authorization,
        &records,
        required_scope,
        OffsetDateTime::now_utc(),
    )?;
    let _ = sqlx::query("UPDATE control.api_tokens SET last_used_at = now() WHERE token_id = $1")
        .bind(record.token_id)
        .execute(pool)
        .await;
    Ok(AuthenticatedToken {
        project_id: record.project_id,
        scopes: record.scopes.clone(),
    })
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(token: &str) -> ApiTokenRecord {
        ApiTokenRecord {
            token_id: Uuid::nil(),
            project_id: Uuid::nil(),
            token_hash: hash_token(token),
            scopes: vec!["ingest:write".into()],
            expires_at: None,
            revoked_at: None,
        }
    }

    #[test]
    fn accepts_valid_scoped_token_and_rejects_lifecycle_failures() {
        let now = OffsetDateTime::UNIX_EPOCH;
        let valid = record("secret");
        assert!(
            authorize(
                Some("Bearer secret"),
                std::slice::from_ref(&valid),
                "ingest:write",
                now
            )
            .is_ok()
        );
        assert_eq!(
            authorize(None, std::slice::from_ref(&valid), "ingest:write", now),
            Err(AuthError::MissingBearer)
        );
        assert_eq!(
            authorize(
                Some("Bearer secret"),
                std::slice::from_ref(&valid),
                "admin",
                now
            ),
            Err(AuthError::MissingScope)
        );
        let mut revoked = valid.clone();
        revoked.revoked_at = Some(now);
        assert_eq!(
            authorize(Some("Bearer secret"), &[revoked], "ingest:write", now),
            Err(AuthError::RevokedToken)
        );
        let mut expired = valid.clone();
        expired.expires_at = Some(now);
        assert_eq!(
            authorize(Some("Bearer secret"), &[expired], "ingest:write", now),
            Err(AuthError::ExpiredToken)
        );
        assert_eq!(
            authorize(
                Some("Basic secret"),
                std::slice::from_ref(&valid),
                "ingest:write",
                now
            ),
            Err(AuthError::InvalidBearer)
        );
        assert_eq!(
            authorize(
                Some("Bearer secret with-space"),
                std::slice::from_ref(&valid),
                "ingest:write",
                now
            ),
            Err(AuthError::InvalidBearer)
        );
        assert_eq!(
            authorize(
                Some("Bearer other"),
                std::slice::from_ref(&valid),
                "ingest:write",
                now
            ),
            Err(AuthError::UnknownToken)
        );
    }
}
