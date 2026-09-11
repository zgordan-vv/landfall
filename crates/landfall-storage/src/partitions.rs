//! Daily raw-event partition creation and retention primitives.

use sqlx::PgPool;
use time::{Date, Duration};

fn partition_suffix(day: Date) -> String {
    format!("{:04}{:02}{:02}", day.year(), day.month() as u8, day.day())
}

fn partition_name(day: Date) -> String {
    format!("raw_events_{}", partition_suffix(day))
}

/// Creates the daily partition covering `[day, day + 1 day)`.
pub async fn ensure_raw_event_partition(pool: &PgPool, day: Date) -> Result<(), sqlx::Error> {
    let next_day = day + Duration::days(1);
    let name = partition_name(day);
    let sql = format!(
        "CREATE TABLE IF NOT EXISTS telemetry.{name} PARTITION OF telemetry.raw_events FOR VALUES FROM ('{day}') TO ('{next_day}')"
    );
    sqlx::query(&sql).execute(pool).await?;
    Ok(())
}

/// Drops one daily raw-event partition. The generated identifier is date-only.
pub async fn drop_raw_event_partition(pool: &PgPool, day: Date) -> Result<(), sqlx::Error> {
    let name = partition_name(day);
    let sql = format!("DROP TABLE IF EXISTS telemetry.{name}");
    sqlx::query(&sql).execute(pool).await?;
    Ok(())
}

/// Drops partitions older than `cutoff`, returning the number removed.
pub async fn drop_raw_event_partitions_before(
    pool: &PgPool,
    cutoff: Date,
) -> Result<u64, sqlx::Error> {
    let names: Vec<String> = sqlx::query_scalar(
        "SELECT c.relname FROM pg_inherits i
         JOIN pg_class c ON c.oid = i.inhrelid
         JOIN pg_class p ON p.oid = i.inhparent
         JOIN pg_namespace n ON n.oid = c.relnamespace
         WHERE n.nspname = 'telemetry' AND p.relname = 'raw_events'
           AND c.relname LIKE 'raw_events_%'",
    )
    .fetch_all(pool)
    .await?;

    let cutoff_suffix = partition_suffix(cutoff);
    let mut removed = 0;
    for name in names {
        let Some(suffix) = name.strip_prefix("raw_events_") else {
            continue;
        };
        if suffix.len() == 8
            && suffix.chars().all(|character| character.is_ascii_digit())
            && suffix < cutoff_suffix.as_str()
        {
            sqlx::query(&format!("DROP TABLE IF EXISTS telemetry.{name}"))
                .execute(pool)
                .await?;
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::{partition_name, partition_suffix};
    use time::Date;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn partition_names_are_deterministic_and_date_only() {
        let day = Date::from_calendar_date(2026, time::Month::September, 11).unwrap();
        assert_eq!(partition_suffix(day), "20260911");
        assert_eq!(partition_name(day), "raw_events_20260911");
    }
}
