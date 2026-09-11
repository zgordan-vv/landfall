//! Retention planning and partition-worker safety semantics.

use time::{Date, Duration};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RetentionPlan {
    pub cutoff: Date,
    pub partitions_to_drop: Vec<Date>,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RetentionError {
    ZeroRetention,
}

/// Computes old daily partitions without performing destructive I/O.
pub fn plan_retention(
    today: Date,
    retention_days: u32,
    existing: impl IntoIterator<Item = Date>,
    dry_run: bool,
) -> Result<RetentionPlan, RetentionError> {
    if retention_days == 0 {
        return Err(RetentionError::ZeroRetention);
    }
    let cutoff = today - Duration::days(i64::from(retention_days));
    let mut partitions_to_drop: Vec<_> = existing.into_iter().filter(|day| *day < cutoff).collect();
    partitions_to_drop.sort_unstable();
    Ok(RetentionPlan {
        cutoff,
        partitions_to_drop,
        dry_run,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dry_run_lists_only_partitions_before_safety_cutoff() {
        let today = Date::from_calendar_date(2026, time::Month::September, 11).unwrap();
        let plan = plan_retention(
            today,
            30,
            [
                today - Duration::days(31),
                today - Duration::days(30),
                today,
            ],
            true,
        )
        .unwrap();
        assert_eq!(plan.partitions_to_drop, vec![today - Duration::days(31)]);
        assert!(plan.dry_run);
        assert_eq!(
            plan_retention(today, 0, [], true),
            Err(RetentionError::ZeroRetention)
        );
    }
}
