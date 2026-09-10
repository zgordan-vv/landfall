//! Streaming NDJSON ingestion primitives used by the Landfall CLI.

use std::{collections::BTreeMap, io::BufRead};

use landfall_core::{
    grouping::{TraceGrouping, TraceRelationship, classify_trace_relationship, group_trace},
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
};
use landfall_protocol::{TraceId, WireEvent};

/// Error while reading or validating one NDJSON record.
#[derive(Debug)]
pub enum IngestError {
    /// Underlying reader failure.
    Io(std::io::Error),
    /// Record was not valid JSON or a known wire event.
    Json {
        /// One-based input line number.
        line: usize,
        /// JSON decoder error.
        source: serde_json::Error,
    },
    /// Event decoded but failed cross-field semantic validation.
    Semantics {
        /// One-based input line number.
        line: usize,
        /// Protocol semantic validation error.
        source: landfall_protocol::WireValueError,
    },
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "read input: {e}"),
            Self::Json { line, source } => write!(f, "line {line}: invalid event JSON: {source}"),
            Self::Semantics { line, source } => {
                write!(f, "line {line}: invalid event semantics: {source}")
            }
        }
    }
}
impl std::error::Error for IngestError {}

/// Reads and validates events one line at a time, retaining only decoded events.
///
/// Prefer [`stream_ndjson`] for production-sized inputs when the caller does not
/// need to retain the complete event set.
pub fn ingest_ndjson<R: BufRead>(reader: R) -> Result<Vec<WireEvent>, IngestError> {
    let mut events = Vec::new();
    stream_ndjson(reader, |event| {
        events.push(event);
        Ok(())
    })?;
    Ok(events)
}

/// Streams validated events to `sink` without accumulating the input.
pub fn stream_ndjson<R, F>(reader: R, mut sink: F) -> Result<usize, IngestError>
where
    R: BufRead,
    F: FnMut(WireEvent) -> Result<(), IngestError>,
{
    let mut accepted = 0;
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(IngestError::Io)?;
        if line.trim().is_empty() {
            continue;
        }
        let event: WireEvent = serde_json::from_str(&line).map_err(|source| IngestError::Json {
            line: index + 1,
            source,
        })?;
        validate_event(&event).map_err(|source| IngestError::Semantics {
            line: index + 1,
            source,
        })?;
        sink(event)?;
        accepted += 1;
    }
    Ok(accepted)
}

/// In-memory canonical trace groups and explicitly proven alias relationships.
#[derive(Debug)]
pub struct InMemoryTraceGroups {
    /// One deterministic grouping per trace identity.
    pub traces: BTreeMap<TraceId, TraceGrouping>,
    /// Pairwise relationships proven by signed identity evidence.
    pub aliases: Vec<(TraceId, TraceId, TraceRelationship)>,
}

/// Canonicalizes and groups a bounded in-memory event set by trace identity.
pub fn group_traces(
    events: Vec<WireEvent>,
) -> Result<InMemoryTraceGroups, Box<dyn std::error::Error>> {
    let mut by_trace = BTreeMap::<TraceId, Vec<WireEvent>>::new();
    for event in events {
        let trace = trace_id(&event).ok_or("event is missing trace_id")?;
        by_trace.entry(trace).or_default().push(event);
    }
    let mut traces = BTreeMap::new();
    for (trace, events) in by_trace {
        let ordered = canonical_order(
            events.into_iter().map(|event| {
                let received_at = occurred_at(&event);
                CollectedEvent::new(event, received_at)
            }),
            OrderingConfig::default(),
        )?;
        traces.insert(trace, group_trace(&ordered)?);
    }
    let values = traces.values().collect::<Vec<_>>();
    let mut aliases = Vec::new();
    for (index, left) in values.iter().enumerate() {
        for right in values.iter().skip(index + 1) {
            let relation = classify_trace_relationship(left, right)?;
            if !matches!(
                relation,
                TraceRelationship::Unrelated | TraceRelationship::GroupingUnavailable
            ) {
                aliases.push((left.trace_id(), right.trace_id(), relation));
            }
        }
    }
    Ok(InMemoryTraceGroups { traces, aliases })
}

fn trace_id(event: &WireEvent) -> Option<TraceId> {
    macro_rules! field { ($($variant:ident),+ $(,)?) => { match event { $(WireEvent::$variant(value) => value.trace_id,)+ } }; }
    field!(
        TraceCreated,
        BlockhashAcquired,
        SimulationStarted,
        SimulationCompleted,
        SigningStarted,
        SigningCompleted,
        SubmissionStarted,
        SubmissionCompleted,
        SubmissionRetryScheduled,
        ConfirmationWaitStarted,
        ConfirmationWaitCompleted,
        StatusObserved,
        ExecutionEnriched,
        BusinessOutcomeObserved,
        DataQualityDetected
    )
}

fn occurred_at(event: &WireEvent) -> landfall_protocol::UtcTimestamp {
    macro_rules! field { ($($variant:ident),+ $(,)?) => { match event { $(WireEvent::$variant(value) => value.occurred_at,)+ } }; }
    field!(
        TraceCreated,
        BlockhashAcquired,
        SimulationStarted,
        SimulationCompleted,
        SigningStarted,
        SigningCompleted,
        SubmissionStarted,
        SubmissionCompleted,
        SubmissionRetryScheduled,
        ConfirmationWaitStarted,
        ConfirmationWaitCompleted,
        StatusObserved,
        ExecutionEnriched,
        BusinessOutcomeObserved,
        DataQualityDetected
    )
}

fn validate_event(event: &WireEvent) -> Result<(), landfall_protocol::WireValueError> {
    macro_rules! validate { ($($variant:ident),+ $(,)?) => { match event { $(WireEvent::$variant(value) => value.validate_semantics(),)+ } }; }
    validate!(
        TraceCreated,
        BlockhashAcquired,
        SimulationStarted,
        SimulationCompleted,
        SigningStarted,
        SigningCompleted,
        SubmissionStarted,
        SubmissionCompleted,
        SubmissionRetryScheduled,
        ConfirmationWaitStarted,
        ConfirmationWaitCompleted,
        StatusObserved,
        ExecutionEnriched,
        BusinessOutcomeObserved,
        DataQualityDetected
    )
}
