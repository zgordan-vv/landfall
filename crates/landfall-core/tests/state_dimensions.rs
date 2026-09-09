//! Orthogonal transaction-state behavior and impossible combinations.

use std::str::FromStr;

use landfall_core::domain::{
    ApplicationOutcome, BusinessActionId, EnvironmentId, EvidenceSet, ExecutionState, LandingState,
    LifecycleStage, ObservationCompleteness, StateInvariantError, TraceId, TraceState,
    TransactionTrace,
};
use landfall_protocol::{BusinessOutcome, Commitment, EventId, ExecutionResult};

const ENVIRONMENT: &str = "0198ef00-0000-7000-8000-000000000200";
const BUSINESS_ACTION: &str = "0198ef10-0008-7000-8000-000000000200";
const TRACE: &str = "0198ef10-0008-7000-8000-000000000101";
const EVENT: &str = "0198ef10-0008-7000-8000-000000000011";

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::error::Error + 'static,
{
    Ok(value.parse()?)
}

#[test]
fn default_trace_is_in_flight_without_invented_outcomes() -> Result<(), Box<dyn std::error::Error>>
{
    let trace = TransactionTrace::new(
        parse::<TraceId>(TRACE)?,
        parse::<EnvironmentId>(ENVIRONMENT)?,
        Some(parse::<BusinessActionId>(BUSINESS_ACTION)?),
        parse::<EventId>(EVENT)?,
    );

    assert_eq!(trace.state(), TraceState::default());
    assert!(!trace.state().observation.is_terminal());
    assert!(!trace.state().observation.is_metric_eligible());
    Ok(())
}

#[test]
fn landing_execution_and_application_outcome_remain_independent() {
    let execution_failed_after_landing = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Confirmed,
        execution: ExecutionState::Failure,
        application: ApplicationOutcome::Unknown,
        observation: ObservationCompleteness::Complete,
    };
    assert_eq!(execution_failed_after_landing.validate(), Ok(()));
    assert_eq!(
        execution_failed_after_landing.landing.commitment(),
        Some(Commitment::Confirmed)
    );

    let network_success_application_failure = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Finalized,
        execution: ExecutionState::Success,
        application: ApplicationOutcome::Failure,
        observation: ObservationCompleteness::Complete,
    };
    assert_eq!(network_success_application_failure.validate(), Ok(()));
}

#[test]
fn timeout_does_not_claim_non_inclusion() {
    let timed_out_while_observation_continues = TraceState {
        lifecycle: LifecycleStage::Submitted,
        landing: LandingState::NotObserved,
        execution: ExecutionState::Unknown,
        application: ApplicationOutcome::Timeout,
        observation: ObservationCompleteness::InProgress,
    };

    assert_eq!(timed_out_while_observation_continues.validate(), Ok(()));
    assert!(!timed_out_while_observation_continues.landing.is_landed());
}

#[test]
fn protocol_results_map_without_collapsing_unknown() {
    assert_eq!(
        ExecutionState::from(ExecutionResult::Success),
        ExecutionState::Success
    );
    assert_eq!(
        ApplicationOutcome::from(BusinessOutcome::Cancelled),
        ApplicationOutcome::Cancelled
    );
    assert_eq!(
        ApplicationOutcome::from(BusinessOutcome::Unknown),
        ApplicationOutcome::Unknown
    );
}

#[test]
fn universally_impossible_combinations_are_rejected() {
    let landing_without_observation = TraceState {
        lifecycle: LifecycleStage::Submitted,
        landing: LandingState::Processed,
        ..TraceState::default()
    };
    assert_eq!(
        landing_without_observation.validate(),
        Err(StateInvariantError::LandingWithoutObservation)
    );

    let execution_without_inclusion = TraceState {
        lifecycle: LifecycleStage::Observed,
        execution: ExecutionState::Success,
        ..TraceState::default()
    };
    assert_eq!(
        execution_without_inclusion.validate(),
        Err(StateInvariantError::ExecutionWithoutInclusion)
    );

    let premature_expiration = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Expired,
        observation: ObservationCompleteness::InProgress,
        ..TraceState::default()
    };
    assert_eq!(
        premature_expiration.validate(),
        Err(StateInvariantError::ExpirationWithoutCompleteObservation)
    );

    let mismatched_incomplete = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Incomplete,
        observation: ObservationCompleteness::Complete,
        ..TraceState::default()
    };
    assert_eq!(
        mismatched_incomplete.validate(),
        Err(StateInvariantError::IncompleteLandingWithoutIncompleteObservation)
    );
}

#[test]
fn validated_state_can_be_attached_to_trace() -> Result<(), Box<dyn std::error::Error>> {
    let event_id = parse::<EventId>(EVENT)?;
    let state = TraceState {
        lifecycle: LifecycleStage::Observed,
        landing: LandingState::Processed,
        execution: ExecutionState::Success,
        application: ApplicationOutcome::Success,
        observation: ObservationCompleteness::InProgress,
    };
    let trace = TransactionTrace::with_state(
        parse(TRACE)?,
        parse(ENVIRONMENT)?,
        Some(parse(BUSINESS_ACTION)?),
        EvidenceSet::new(event_id),
        state,
    )?;

    assert_eq!(trace.state(), state);
    Ok(())
}
