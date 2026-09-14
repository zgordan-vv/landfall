//! Streaming NDJSON ingestion primitives used by the Landfall CLI.

use std::{collections::BTreeMap, io::BufRead};

/// Supported operator commands beyond NDJSON ingestion.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum OperatorCommand {
    /// Initialize local operator configuration.
    Init,
    /// Inspect local configuration and dependency health.
    Doctor,
    /// Inspect a transaction trace.
    Trace,
    /// Operate on a named report subcommand.
    Report(String),
    /// Inspect the active diagnostic rule set.
    Rules,
    /// Preview retention without deleting retained data.
    RetentionDryRun,
    /// Run the local product demonstration.
    Demo,
}

/// Parses the stable operator command surface.
pub fn parse_operator_command(args: &[String]) -> Result<OperatorCommand, &'static str> {
    match args {
        [command] if command == "init" => Ok(OperatorCommand::Init),
        [command] if command == "doctor" => Ok(OperatorCommand::Doctor),
        [command] if command == "trace" => Ok(OperatorCommand::Trace),
        [command, sub] if command == "report" => Ok(OperatorCommand::Report(sub.clone())),
        [command] if command == "rules" => Ok(OperatorCommand::Rules),
        [command, run, dry] if command == "retention" && run == "run" && dry == "--dry-run" => {
            Ok(OperatorCommand::RetentionDryRun)
        }
        [command] if command == "demo" => Ok(OperatorCommand::Demo),
        _ => Err(
            "usage: init | doctor | trace | report <create|status|download> | rules | retention run --dry-run | demo",
        ),
    }
}

#[cfg(test)]
mod operator_tests {
    use super::*;
    #[test]
    fn parses_operator_commands() {
        assert_eq!(
            parse_operator_command(&["init".into()]),
            Ok(OperatorCommand::Init)
        );
        assert_eq!(
            parse_operator_command(&["report".into(), "status".into()]),
            Ok(OperatorCommand::Report("status".into()))
        );
        assert!(parse_operator_command(&["unknown".into()]).is_err());
    }
}

use landfall_core::{
    data_quality::evaluate_data_quality,
    diagnostics::{
        ProbableDiagnosticConfig, evaluate_confirmed_diagnostics, evaluate_probable_diagnostics,
        evaluate_unknown_diagnostics,
    },
    grouping::{TraceGrouping, TraceRelationship, classify_trace_relationship, group_trace},
    metrics::{TraceMetricFlags, trace_metric_flags},
    ordering::{CollectedEvent, OrderingConfig, canonical_order},
    recommendations::{AdvisoryRecommendation, generate_recommendations},
    reducer::{TraceProjection, reduce_trace},
};
use landfall_protocol::{TraceId, WireEvent};

/// Selector accepted by the reproject command.
#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(missing_docs)]
pub enum ReprojectSelector {
    Trace(TraceId),
    TimeRange { from: String, until: String },
    RuleSetVersion(String),
}

/// Parses exactly one reproject scope from CLI-style arguments.
pub fn parse_reproject_selector(args: &[String]) -> Result<ReprojectSelector, &'static str> {
    if args.len() == 2 && args[0] == "--trace" {
        return args[1]
            .parse()
            .map(ReprojectSelector::Trace)
            .map_err(|_| "invalid trace id");
    }
    if args.len() == 4 && args[0] == "--from" && args[2] == "--until" {
        if args[1].is_empty() || args[3].is_empty() {
            return Err("time range values must not be empty");
        }
        return Ok(ReprojectSelector::TimeRange {
            from: args[1].clone(),
            until: args[3].clone(),
        });
    }
    if args.len() == 2 && args[0] == "--rule-set-version" && !args[1].is_empty() {
        return Ok(ReprojectSelector::RuleSetVersion(args[1].clone()));
    }
    Err(
        "choose exactly one: --trace ID, --from RFC3339 --until RFC3339, or --rule-set-version VERSION",
    )
}

#[cfg(test)]
mod reproject_tests {
    use super::{ReprojectSelector, parse_reproject_selector};
    #[test]
    fn accepts_one_scope_and_rejects_ambiguous_input() {
        let trace = "0198ef00-0000-7000-8000-000000000300".to_owned();
        assert!(matches!(
            parse_reproject_selector(&["--trace".into(), trace]),
            Ok(ReprojectSelector::Trace(_))
        ));
        assert!(
            parse_reproject_selector(&[
                "--trace".into(),
                "id".into(),
                "--rule-set-version".into(),
                "v1".into()
            ])
            .is_err()
        );
    }
}

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
    /// Derived offline analysis products by trace.
    pub analyses: BTreeMap<TraceId, TraceAnalysis>,
}

/// Analysis products generated for one canonical trace.
#[derive(Debug)]
pub struct TraceAnalysis {
    /// Reduced state projection.
    pub projection: TraceProjection,
    /// Number of data-quality findings.
    pub data_quality_findings: usize,
    /// Number of confirmed diagnostic findings.
    pub confirmed_diagnostics: usize,
    /// Number of probable diagnostic findings.
    pub probable_diagnostics: usize,
    /// Number of unknown diagnostic findings.
    pub unknown_diagnostics: usize,
    /// Evidence-linked advisory recommendations.
    pub recommendations: Vec<AdvisoryRecommendation>,
    /// Pure metric flags.
    pub metrics: TraceMetricFlags,
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
    let mut analyses = BTreeMap::new();
    for (trace, events) in by_trace {
        let ordered = canonical_order(
            events.into_iter().map(|event| {
                let received_at = occurred_at(&event);
                CollectedEvent::new(event, received_at)
            }),
            OrderingConfig::default(),
        )?;
        let grouping = group_trace(&ordered)?;
        let projection = reduce_trace(&ordered)?;
        let quality = evaluate_data_quality(&ordered, &projection, &grouping)?;
        let confirmed = evaluate_confirmed_diagnostics(&ordered, &projection);
        let probable = evaluate_probable_diagnostics(
            &ordered,
            &projection,
            &grouping,
            ProbableDiagnosticConfig::default(),
        );
        let unknown = evaluate_unknown_diagnostics(&ordered, &projection, &grouping);
        let mut findings = confirmed.clone();
        findings.extend(probable.clone());
        findings.extend(unknown.clone());
        let recommendations = generate_recommendations(trace, &findings);
        analyses.insert(
            trace,
            TraceAnalysis {
                metrics: trace_metric_flags(&projection),
                projection,
                data_quality_findings: quality.findings().len(),
                confirmed_diagnostics: confirmed.len(),
                probable_diagnostics: probable.len(),
                unknown_diagnostics: unknown.len(),
                recommendations,
            },
        );
        traces.insert(trace, grouping);
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
    Ok(InMemoryTraceGroups {
        traces,
        aliases,
        analyses,
    })
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
