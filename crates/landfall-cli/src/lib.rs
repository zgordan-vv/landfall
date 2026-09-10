//! Streaming NDJSON ingestion primitives used by the Landfall CLI.

use std::io::BufRead;

use landfall_protocol::WireEvent;

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
pub fn ingest_ndjson<R: BufRead>(reader: R) -> Result<Vec<WireEvent>, IngestError> {
    let mut events = Vec::new();
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
        events.push(event);
    }
    Ok(events)
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
