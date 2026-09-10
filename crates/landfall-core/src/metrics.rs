//! Pure metric definitions over reduced trace projections.
#![allow(clippy::struct_excessive_bools)]

use crate::{
    domain::{ApplicationOutcome, ExecutionState, LandingState, ObservationCompleteness},
    reducer::TraceProjection,
};

/// Stable version of the metric semantics.
pub const METRIC_DEFINITIONS_VERSION: &str = "metric-definitions-v1";

/// Per-trace metric flags. Aggregators can sum each boolean independently.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceMetricFlags {
    /// Whether the trace belongs in terminal denominators.
    pub terminal_eligible: bool,
    /// Whether network inclusion was observed without conflict.
    pub landed: bool,
    /// Whether on-chain execution succeeded.
    pub execution_success: bool,
    /// Whether on-chain execution failed.
    pub execution_failure: bool,
    /// Whether the application reported success.
    pub application_success: bool,
}

/// Derives metric flags without storage, clocks, or side effects.
#[must_use]
pub const fn trace_metric_flags(projection: &TraceProjection) -> TraceMetricFlags {
    let state = projection.trace().state();
    TraceMetricFlags {
        terminal_eligible: state.observation.is_metric_eligible(),
        landed: state.landing.is_landed(),
        execution_success: matches!(state.execution, ExecutionState::Success),
        execution_failure: matches!(state.execution, ExecutionState::Failure),
        application_success: matches!(state.application, ApplicationOutcome::Success),
    }
}

/// Returns whether a projection can enter a terminal metric denominator.
#[must_use]
pub const fn terminal_denominator_eligible(projection: &TraceProjection) -> bool {
    matches!(
        projection.trace().state().observation,
        ObservationCompleteness::Complete
    )
}

/// Returns whether the projection has an unambiguous inclusion commitment.
#[must_use]
pub const fn landed_numerator(projection: &TraceProjection) -> bool {
    matches!(
        projection.trace().state().landing,
        LandingState::Processed | LandingState::Confirmed | LandingState::Finalized
    )
}
