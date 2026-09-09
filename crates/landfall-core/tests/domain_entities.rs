//! Domain entity identity and relationship invariants.

use std::str::FromStr;

use landfall_core::domain::{
    AnalysisScope, BusinessAction, BusinessActionId, CohortId, Diagnostic, DiagnosticId,
    DomainInvariantError, EnvironmentId, EvidenceSet, ExecutionMetadata, LifecycleEvidence,
    ObserverSourceId, OperationId, Recommendation, RecommendationId, Simulation, SimulationId,
    StatusObservation, SubmissionAttempt, TraceId, TransactionTrace,
};
use landfall_protocol::{AttemptId, EventId};

const ENVIRONMENT_A: &str = "0198ef00-0000-7000-8000-000000000200";
const ENVIRONMENT_B: &str = "0198ef00-0000-7000-8000-000000000201";
const BUSINESS_ACTION: &str = "0198ef10-0008-7000-8000-000000000200";
const TRACE: &str = "0198ef10-0008-7000-8000-000000000101";
const ATTEMPT: &str = "0198ef10-0008-7000-8000-000000000501";
const OPERATION: &str = "0198ef10-0008-7000-8000-000000000301";
const OBSERVER: &str = "0198ef00-0000-7000-8000-000000000500";
const EVENT_A: &str = "0198ef10-0008-7000-8000-000000000011";
const EVENT_B: &str = "0198ef10-0008-7000-8000-000000000012";
const DIAGNOSTIC: &str = "0198ef10-0008-7000-8000-000000000601";
const RECOMMENDATION: &str = "0198ef10-0008-7000-8000-000000000701";
const COHORT: &str = "0198ef10-0008-7000-8000-000000000801";

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::error::Error + 'static,
{
    Ok(value.parse()?)
}

#[test]
fn domain_ids_require_canonical_uuid_v7() -> Result<(), Box<dyn std::error::Error>> {
    let diagnostic = parse::<DiagnosticId>(DIAGNOSTIC)?;
    let recommendation = parse::<RecommendationId>(RECOMMENDATION)?;
    let cohort = parse::<CohortId>(COHORT)?;

    assert_eq!(diagnostic.to_string(), DIAGNOSTIC);
    assert_eq!(DiagnosticId::try_from(diagnostic.into_uuid())?, diagnostic);
    assert_eq!(recommendation.to_string(), RECOMMENDATION);
    assert_eq!(cohort.to_string(), COHORT);
    assert!(DiagnosticId::from_str("550e8400-e29b-41d4-a716-446655440000").is_err());
    assert!(DiagnosticId::from_str(&DIAGNOSTIC.to_uppercase()).is_err());
    Ok(())
}

#[test]
fn evidence_is_non_empty_deduplicated_and_not_semantically_ordered()
-> Result<(), Box<dyn std::error::Error>> {
    let event_a = parse::<EventId>(EVENT_A)?;
    let event_b = parse::<EventId>(EVENT_B)?;
    let evidence = EvidenceSet::try_from_events([event_b, event_a, event_a], "trace")?;

    assert_eq!(evidence.len(), 2);
    assert!(evidence.contains(&event_a));
    assert_eq!(
        EvidenceSet::try_from_events([], "diagnostic"),
        Err(DomainInvariantError::EmptyEvidence {
            entity: "diagnostic"
        })
    );
    Ok(())
}

#[test]
fn lifecycle_entities_preserve_missing_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    let event_a = parse::<EventId>(EVENT_A)?;
    let event_b = parse::<EventId>(EVENT_B)?;
    let trace_id = parse::<TraceId>(TRACE)?;
    let simulation_id = SimulationId::new(parse::<OperationId>(OPERATION)?);
    let attempt_id = parse::<AttemptId>(ATTEMPT)?;

    let completion_only = LifecycleEvidence::new(None, Some(event_b))?;
    let simulation = Simulation::new(simulation_id, trace_id, completion_only);
    assert_eq!(simulation.evidence().started_event_id(), None);
    assert_eq!(simulation.evidence().completed_event_id(), Some(event_b));

    let both = LifecycleEvidence::new(Some(event_a), Some(event_b))?;
    let attempt = SubmissionAttempt::new(attempt_id, trace_id, both);
    assert_eq!(attempt.evidence(), both);

    assert_eq!(
        LifecycleEvidence::new(None, None),
        Err(DomainInvariantError::EmptyEvidence {
            entity: "lifecycle operation"
        })
    );
    assert_eq!(
        LifecycleEvidence::new(Some(event_a), Some(event_a)),
        Err(DomainInvariantError::ReusedLifecycleEvent)
    );
    Ok(())
}

#[test]
fn business_action_links_only_explicit_same_environment_traces()
-> Result<(), Box<dyn std::error::Error>> {
    let action_id = parse::<BusinessActionId>(BUSINESS_ACTION)?;
    let environment_a = parse::<EnvironmentId>(ENVIRONMENT_A)?;
    let event_a = parse::<EventId>(EVENT_A)?;
    let mut action = BusinessAction::new(action_id, environment_a, event_a);

    let matching = TransactionTrace::new(
        parse(TRACE)?,
        environment_a,
        Some(action_id),
        parse(EVENT_B)?,
    );
    assert_eq!(action.link_trace(&matching), Ok(true));
    assert_eq!(action.link_trace(&matching), Ok(false));

    let wrong_action = TransactionTrace::new(parse(TRACE)?, environment_a, None, event_a);
    assert_eq!(
        action.link_trace(&wrong_action),
        Err(DomainInvariantError::BusinessActionMismatch)
    );

    let wrong_environment = TransactionTrace::new(
        parse(TRACE)?,
        parse(ENVIRONMENT_B)?,
        Some(action_id),
        event_a,
    );
    assert_eq!(
        action.link_trace(&wrong_environment),
        Err(DomainInvariantError::EnvironmentMismatch)
    );
    Ok(())
}

#[test]
fn observations_diagnostics_and_recommendations_keep_evidence_links()
-> Result<(), Box<dyn std::error::Error>> {
    let trace_id = parse::<TraceId>(TRACE)?;
    let observer_source_id = parse::<ObserverSourceId>(OBSERVER)?;
    let event_a = parse::<EventId>(EVENT_A)?;
    let event_b = parse::<EventId>(EVENT_B)?;

    let observation = StatusObservation::new(trace_id, observer_source_id, event_a);
    let execution = ExecutionMetadata::new(trace_id, observer_source_id, event_b);
    assert_eq!(observation.id().source_id(), event_a);
    assert_eq!(execution.id().source_id(), event_b);

    let evidence = EvidenceSet::try_from_events([event_a, event_b], "diagnostic")?;
    let diagnostic = Diagnostic::new(
        parse(DIAGNOSTIC)?,
        AnalysisScope::Trace(trace_id),
        evidence.clone(),
    );
    let recommendation = Recommendation::new(
        parse(RECOMMENDATION)?,
        AnalysisScope::Trace(trace_id),
        [diagnostic.id()],
        evidence,
    );

    assert_eq!(recommendation.scope(), diagnostic.scope());
    assert_eq!(
        recommendation.diagnostic_ids().copied().collect::<Vec<_>>(),
        [diagnostic.id()]
    );
    assert_eq!(recommendation.evidence().len(), 2);
    Ok(())
}
