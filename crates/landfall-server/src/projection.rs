//! Adapter from persisted rows to the pure ordering/reducer pipeline.

use landfall_core::{
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
    reducer::{ReducerError, TraceProjection, reduce_trace},
};
use landfall_protocol::{UtcTimestamp, WireEvent};
use landfall_storage::RawEventRow;
use time::format_description::well_known::Rfc3339;

#[derive(Debug)]
pub enum ProjectionError {
    InvalidEvent(serde_json::Error),
    InvalidTimestamp(String),
    Ordering(landfall_core::ordering::OrderingError),
    Reduction(ReducerError),
}

/// Converts canonical database rows into the deterministic reducer result.
pub fn reduce_loaded_events(
    rows: impl IntoIterator<Item = RawEventRow>,
) -> Result<TraceProjection, ProjectionError> {
    let events = rows
        .into_iter()
        .map(|row| {
            let event: WireEvent =
                serde_json::from_value(row.payload).map_err(ProjectionError::InvalidEvent)?;
            let timestamp = row
                .received_at
                .format(&Rfc3339)
                .map_err(|error| ProjectionError::InvalidTimestamp(error.to_string()))?;
            let received_at = timestamp
                .parse::<UtcTimestamp>()
                .map_err(|error| ProjectionError::InvalidTimestamp(error.to_string()))?;
            Ok(CollectedEvent::new(event, received_at))
        })
        .collect::<Result<Vec<_>, ProjectionError>>()?;
    let ordered =
        canonical_order(events, OrderingConfig::default()).map_err(ProjectionError::Ordering)?;
    reduce_trace(&ordered).map_err(ProjectionError::Reduction)
}
