use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TraceListFilter {
    pub project_id: Option<String>,
    pub environment_id: Option<String>,
    pub status: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TraceFilterError {
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("time range must be non-negative and at most 31 days")]
    InvalidRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedTraceFilter {
    pub project_id: Option<String>,
    pub environment_id: Option<String>,
    pub status: Option<String>,
    pub from: Option<OffsetDateTime>,
    pub to: Option<OffsetDateTime>,
}

pub fn validate_trace_filter(
    filter: TraceListFilter,
) -> Result<ValidatedTraceFilter, TraceFilterError> {
    let from = parse(filter.from)?;
    let to = parse(filter.to)?;
    if let (Some(from), Some(to)) = (from, to) {
        let range = to - from;
        if range.is_negative() || range > time::Duration::days(31) {
            return Err(TraceFilterError::InvalidRange);
        }
    }
    Ok(ValidatedTraceFilter {
        project_id: filter.project_id,
        environment_id: filter.environment_id,
        status: filter.status,
        from,
        to,
    })
}

fn parse(value: Option<String>) -> Result<Option<OffsetDateTime>, TraceFilterError> {
    value
        .map(|text| {
            OffsetDateTime::parse(&text, &Rfc3339).map_err(|_| TraceFilterError::InvalidTimestamp)
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_bounded_range_and_rejects_unbounded_range() {
        let valid = validate_trace_filter(TraceListFilter {
            from: Some("2026-01-01T00:00:00Z".into()),
            to: Some("2026-01-15T00:00:00Z".into()),
            ..Default::default()
        })
        .expect("valid");
        assert!(valid.from.is_some());
        assert_eq!(
            validate_trace_filter(TraceListFilter {
                from: Some("2026-01-01T00:00:00Z".into()),
                to: Some("2026-03-01T00:00:00Z".into()),
                ..Default::default()
            }),
            Err(TraceFilterError::InvalidRange)
        );
    }
}
