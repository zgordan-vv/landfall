//! Deterministic advisory recommendations linked to diagnostic evidence.

use crate::{
    diagnostics::{DiagnosticClaimKey, DiagnosticFinding},
    domain::{AnalysisScope, DiagnosticId, EvidenceSet, RecommendationId, TraceId},
};

/// Stable semantics version for advisory recommendations.
pub const RECOMMENDATION_RULE_SET_VERSION: &str = "recommendation-rules-v1";

/// One actionable but non-mutating recommendation category.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RecommendationKey {
    /// Increase configured compute headroom after representative simulation.
    IncreaseComputeHeadroom,
    /// Shorten signing path or refresh blockhash closer to submission.
    ReduceSigningDelay,
    /// Fail over or investigate the degraded RPC route.
    ReviewRpcRoute,
    /// Stop retrying after the configured commitment is observed.
    TightenRetryPolicy,
    /// Add the missing instrumentation or observer coverage.
    ImproveEvidenceCoverage,
}

/// Advisory recommendation with direct links to the finding and source events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdvisoryRecommendation {
    id: RecommendationId,
    diagnostic_id: DiagnosticId,
    scope: AnalysisScope,
    key: RecommendationKey,
    rule_set_version: &'static str,
    evidence: EvidenceSet,
}

impl AdvisoryRecommendation {
    /// Recommendation identity.
    #[must_use]
    pub const fn id(&self) -> RecommendationId {
        self.id
    }
    /// Diagnostic that caused this advice.
    #[must_use]
    pub const fn diagnostic_id(&self) -> DiagnosticId {
        self.diagnostic_id
    }
    /// Scope of the advice.
    #[must_use]
    pub const fn scope(&self) -> AnalysisScope {
        self.scope
    }
    /// Machine-readable action category.
    #[must_use]
    pub const fn key(&self) -> RecommendationKey {
        self.key
    }
    /// Recommendation rule-set version.
    #[must_use]
    pub const fn rule_set_version(&self) -> &'static str {
        self.rule_set_version
    }
    /// Immutable events supporting both the finding and advice.
    #[must_use]
    pub const fn evidence(&self) -> &EvidenceSet {
        &self.evidence
    }
}

/// Generates advisory recommendations for one trace's findings.
#[must_use]
pub fn generate_recommendations(
    trace_id: TraceId,
    findings: &[DiagnosticFinding],
) -> Vec<AdvisoryRecommendation> {
    findings
        .iter()
        .filter_map(|finding| {
            let key = key_for(finding.claim_key())?;
            let id = RecommendationId::try_from(finding.id().into_uuid()).ok()?;
            Some(AdvisoryRecommendation {
                id,
                diagnostic_id: finding.id(),
                scope: AnalysisScope::Trace(trace_id),
                key,
                rule_set_version: RECOMMENDATION_RULE_SET_VERSION,
                evidence: finding.evidence().clone(),
            })
        })
        .collect()
}

fn key_for(claim: DiagnosticClaimKey) -> Option<RecommendationKey> {
    Some(match claim {
        DiagnosticClaimKey::ComputeBudgetFailure | DiagnosticClaimKey::LowComputeHeadroom => {
            RecommendationKey::IncreaseComputeHeadroom
        }
        DiagnosticClaimKey::ExcessiveSigningDelay => RecommendationKey::ReduceSigningDelay,
        DiagnosticClaimKey::RouteDegradationSignal => RecommendationKey::ReviewRpcRoute,
        DiagnosticClaimKey::UnsafeRedundantRetry => RecommendationKey::TightenRetryPolicy,
        DiagnosticClaimKey::MissingEvidence => RecommendationKey::ImproveEvidenceCoverage,
        _ => return None,
    })
}
