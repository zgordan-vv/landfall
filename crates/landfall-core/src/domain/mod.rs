//! Domain entities and identity invariants used by deterministic reduction.
//!
//! Wire DTOs remain owned by `landfall-protocol`. This module models the
//! relationships produced from those DTOs without depending on storage, HTTP,
//! RPC, or serialization concerns.

mod entities;
mod evidence;
mod ids;
mod state;

pub use entities::{
    AnalysisScope, BusinessAction, Diagnostic, ExecutionMetadata, Recommendation, Simulation,
    StatusObservation, SubmissionAttempt, TransactionTrace,
};
pub use evidence::{EvidenceSet, LifecycleEvidence};
pub use ids::{
    CohortId, DiagnosticId, DomainIdError, ExecutionMetadataId, RecommendationId, SimulationId,
    StatusObservationId,
};
pub use state::{
    ApplicationOutcome, ExecutionState, LandingState, LifecycleStage, ObservationCompleteness,
    StateInvariantError, TraceState,
};

pub use landfall_protocol::{
    AttemptId, BusinessActionId, EnvironmentId, EventId, ObserverSourceId, OperationId, TraceId,
};

/// Violation of a relationship required by the domain model.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainInvariantError {
    /// An entity that must be evidence-backed received no source event.
    EmptyEvidence {
        /// Name of the entity whose evidence collection was empty.
        entity: &'static str,
    },
    /// One immutable event was incorrectly used as both lifecycle boundaries.
    ReusedLifecycleEvent,
    /// A trace and business action belong to different environments.
    EnvironmentMismatch,
    /// A trace does not explicitly reference the business action being linked.
    BusinessActionMismatch,
}

impl std::fmt::Display for DomainInvariantError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyEvidence { entity } => {
                write!(formatter, "{entity} requires at least one evidence event")
            }
            Self::ReusedLifecycleEvent => formatter
                .write_str("started and completed lifecycle boundaries require distinct event IDs"),
            Self::EnvironmentMismatch => {
                formatter.write_str("related entities belong to different environments")
            }
            Self::BusinessActionMismatch => formatter
                .write_str("trace does not reference the business action it is being linked to"),
        }
    }
}

impl std::error::Error for DomainInvariantError {}
