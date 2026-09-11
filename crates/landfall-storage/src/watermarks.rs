//! Projection version and watermark persistence.

use sqlx::PgPool;
use time::OffsetDateTime;
use uuid::Uuid;

/// Advances a trace watermark only when the supplied version is newer.
pub async fn advance_projection_watermark(
    pool: &PgPool,
    trace_id: Uuid,
    version: i64,
    updated_at: OffsetDateTime,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO work.projection_watermarks (trace_id, projection_version, updated_at) VALUES ($1,$2,$3) ON CONFLICT (trace_id) DO UPDATE SET projection_version=EXCLUDED.projection_version, updated_at=EXCLUDED.updated_at WHERE work.projection_watermarks.projection_version < EXCLUDED.projection_version")
        .bind(trace_id).bind(version).bind(updated_at).execute(pool).await?;
    Ok(())
}
