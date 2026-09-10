//! Version manifest for replayable core semantics.

use crate::{
    diagnostics::DIAGNOSTIC_RULE_SET_VERSION, metrics::METRIC_DEFINITIONS_VERSION,
    recommendations::RECOMMENDATION_RULE_SET_VERSION, reducer::REDUCER_VERSION,
};

/// All semantic versions needed to interpret a derived result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoreVersionManifest {
    /// Trace reducer semantics.
    pub reducer: &'static str,
    /// Confirmed/probable/unknown diagnostic semantics.
    pub diagnostics: &'static str,
    /// Pure metric definitions.
    pub metrics: &'static str,
    /// Advisory recommendation semantics.
    pub recommendations: &'static str,
}

/// Returns the versions used by this build.
#[must_use]
pub const fn current_versions() -> CoreVersionManifest {
    CoreVersionManifest {
        reducer: REDUCER_VERSION,
        diagnostics: DIAGNOSTIC_RULE_SET_VERSION,
        metrics: METRIC_DEFINITIONS_VERSION,
        recommendations: RECOMMENDATION_RULE_SET_VERSION,
    }
}

/// Checks whether two manifests can be compared without semantic translation.
#[must_use]
pub fn same_semantics(left: CoreVersionManifest, right: CoreVersionManifest) -> bool {
    left.reducer == right.reducer
        && left.diagnostics == right.diagnostics
        && left.metrics == right.metrics
        && left.recommendations == right.recommendations
}
