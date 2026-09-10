//! Golden tests for the initial confirmed diagnostic rule set.

use std::str::FromStr;

use landfall_core::{
    diagnostics::{
        DIAGNOSTIC_RULE_SET_VERSION, DiagnosticCertainty, DiagnosticClaimKey, DiagnosticRuleId,
        evaluate_confirmed_diagnostics,
    },
    ordering::{CanonicalOrder, CollectedEvent, OrderingConfig, canonical_order},
    reducer::{TraceProjection, reduce_trace},
};
use landfall_protocol::{
    EventBatch, ExecutionResult, NormalizedErrorCategory, UtcTimestamp, WireEvent,
};

fn parse<T: FromStr>(value: &str) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::error::Error + 'static,
{
    Ok(value.parse()?)
}

fn fixture(name: &str) -> Result<EventBatch, Box<dyn std::error::Error>> {
    let json = match name {
        "simulation-error" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/simulation-error.batch.json"
        ),
        "submission-rejection" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/submission-rejection.batch.json"
        ),
        "compute-budget-error" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/compute-budget-error.batch.json"
        ),
        "expiry-without-inclusion" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/expiry-without-inclusion.batch.json"
        ),
        "timeout-later-success" => include_str!(
            "../../../fixtures/protocol/v1/valid/incidents/timeout-later-success.batch.json"
        ),
        "success" => {
            include_str!("../../../fixtures/protocol/v1/valid/incidents/success.batch.json")
        }
        _ => return Err(std::io::Error::other("unknown diagnostic fixture").into()),
    };
    Ok(serde_json::from_str(json)?)
}

fn order(
    events: impl IntoIterator<Item = WireEvent>,
) -> Result<CanonicalOrder, Box<dyn std::error::Error>> {
    let received_at = parse::<UtcTimestamp>("2026-08-29T12:01:00Z")?;
    Ok(canonical_order(
        events
            .into_iter()
            .map(|event| CollectedEvent::new(event, received_at)),
        OrderingConfig::default(),
    )?)
}

fn project(events: &CanonicalOrder) -> Result<TraceProjection, Box<dyn std::error::Error>> {
    Ok(reduce_trace(events)?)
}

fn evaluate(
    events: impl IntoIterator<Item = WireEvent>,
) -> Result<Vec<landfall_core::diagnostics::DiagnosticFinding>, Box<dyn std::error::Error>> {
    let ordered = order(events)?;
    let projection = project(&ordered)?;
    Ok(evaluate_confirmed_diagnostics(&ordered, &projection))
}

fn only_claim(
    findings: &[landfall_core::diagnostics::DiagnosticFinding],
) -> Result<DiagnosticClaimKey, Box<dyn std::error::Error>> {
    let finding = findings
        .first()
        .ok_or_else(|| std::io::Error::other("missing diagnostic finding"))?;
    assert_eq!(findings.len(), 1);
    assert_eq!(finding.rule_set_version(), DIAGNOSTIC_RULE_SET_VERSION);
    assert_eq!(finding.certainty(), DiagnosticCertainty::Confirmed);
    assert_eq!(finding.evidence().len(), 1);
    Ok(finding.claim_key())
}

#[test]
fn simulation_error_is_confirmed_from_direct_rpc_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let findings = evaluate(fixture("simulation-error")?.events)?;

    assert_eq!(only_claim(&findings)?, DiagnosticClaimKey::SimulationError);
    assert_eq!(findings[0].rule_id(), DiagnosticRuleId::SimulationError);
    assert_eq!(findings[0].rule_id().as_str(), "RULE-SIM-001");
    Ok(())
}

#[test]
fn rpc_rejection_is_confirmed_for_the_observed_route() -> Result<(), Box<dyn std::error::Error>> {
    let findings = evaluate(fixture("submission-rejection")?.events)?;

    assert_eq!(
        only_claim(&findings)?,
        DiagnosticClaimKey::RpcSubmissionRejection
    );
    assert_eq!(
        findings[0].rule_id(),
        DiagnosticRuleId::RpcSubmissionRejection
    );
    Ok(())
}

#[test]
fn non_compute_on_chain_failure_is_a_confirmed_execution_error()
-> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("compute-budget-error")?.events;
    let WireEvent::ExecutionEnriched(event) = &mut events[0] else {
        return Err(std::io::Error::other("execution fixture is missing").into());
    };
    if let Some(error) = &mut event.attributes.error {
        error.category = NormalizedErrorCategory::InstructionError;
    }

    let findings = evaluate(events)?;

    assert_eq!(
        only_claim(&findings)?,
        DiagnosticClaimKey::OnChainExecutionError
    );
    assert_eq!(
        findings[0].rule_id(),
        DiagnosticRuleId::OnChainExecutionError
    );
    Ok(())
}

#[test]
fn compute_budget_failure_is_confirmed_without_generic_duplication()
-> Result<(), Box<dyn std::error::Error>> {
    let findings = evaluate(fixture("compute-budget-error")?.events)?;

    assert_eq!(
        only_claim(&findings)?,
        DiagnosticClaimKey::ComputeBudgetFailure
    );
    assert_eq!(
        findings[0].rule_id(),
        DiagnosticRuleId::ComputeBudgetFailure
    );
    Ok(())
}

#[test]
fn validity_window_passed_without_inclusion_is_confirmed() -> Result<(), Box<dyn std::error::Error>>
{
    let findings = evaluate(fixture("expiry-without-inclusion")?.events)?;

    assert_eq!(
        only_claim(&findings)?,
        DiagnosticClaimKey::ExpiredWithoutObservedInclusion
    );
    assert_eq!(
        findings[0].rule_id(),
        DiagnosticRuleId::ValidityWindowPassed
    );
    Ok(())
}

#[test]
fn client_timeout_followed_by_network_success_is_confirmed_as_separate_claim()
-> Result<(), Box<dyn std::error::Error>> {
    let findings = evaluate(fixture("timeout-later-success")?.events)?;

    assert_eq!(
        only_claim(&findings)?,
        DiagnosticClaimKey::ClientTimeoutFollowedByNetworkSuccess
    );
    assert_eq!(
        findings[0].rule_id(),
        DiagnosticRuleId::ClientTimeoutNetworkSuccess
    );
    Ok(())
}

#[test]
fn successful_execution_without_timeout_produces_no_confirmed_diagnostic()
-> Result<(), Box<dyn std::error::Error>> {
    let findings = evaluate(fixture("success")?.events)?;

    assert!(findings.is_empty());
    Ok(())
}

#[test]
fn successful_status_after_non_timeout_submission_is_not_timeout_later_success()
-> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("timeout-later-success")?.events;
    let WireEvent::SubmissionCompleted(event) = &mut events[0] else {
        return Err(std::io::Error::other("submission fixture is missing").into());
    };
    event.attributes.transport_result = landfall_protocol::TransportResult::ResponseReceived;
    event.attributes.rpc_result = landfall_protocol::SubmissionRpcResult::Accepted;
    event.attributes.error = None;

    let findings = evaluate(events)?;

    assert!(findings.is_empty());
    Ok(())
}

#[test]
fn input_permutation_does_not_change_confirmed_diagnostics()
-> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("timeout-later-success")?.events;
    let baseline = evaluate(events.clone())?;
    events.reverse();

    assert_eq!(evaluate(events)?, baseline);
    Ok(())
}

#[test]
fn successful_on_chain_execution_with_error_absent_is_not_execution_failure()
-> Result<(), Box<dyn std::error::Error>> {
    let mut events = fixture("compute-budget-error")?.events;
    let WireEvent::ExecutionEnriched(event) = &mut events[0] else {
        return Err(std::io::Error::other("execution fixture is missing").into());
    };
    event.attributes.execution_result = ExecutionResult::Success;
    event.attributes.error = None;

    let findings = evaluate(events)?;

    assert!(findings.is_empty());
    Ok(())
}
