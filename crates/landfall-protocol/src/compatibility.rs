//! Exact schema-version and event-type capability negotiation.

use std::fmt;

use thiserror::Error;

/// Exact event wire versions implemented by this crate.
pub const SUPPORTED_SCHEMA_VERSIONS: &[&str] = &["1.0"];

/// Event discriminators implemented for wire version 1.0.
pub const SUPPORTED_EVENT_TYPES: &[&str] = &[
    "landfall.data_quality.detected",
    "solana.blockhash.acquired",
    "solana.business_outcome.observed",
    "solana.confirmation_wait.completed",
    "solana.confirmation_wait.started",
    "solana.execution.enriched",
    "solana.signing.completed",
    "solana.signing.started",
    "solana.simulation.completed",
    "solana.simulation.started",
    "solana.status.observed",
    "solana.submission.completed",
    "solana.submission.retry_scheduled",
    "solana.submission.started",
    "solana.trace.created",
];

/// Stable public reason for rejecting an unsupported producer capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityErrorCode {
    /// No bundled contract exists for the exact declared version.
    UnsupportedSchemaVersion,
    /// The exact version exists but does not register this event discriminator.
    UnsupportedEventType,
}

impl CompatibilityErrorCode {
    /// Stable code suitable for API responses, metrics, and bounded logs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "LF_UNSUPPORTED_SCHEMA_VERSION",
            Self::UnsupportedEventType => "LF_UNSUPPORTED_EVENT_TYPE",
        }
    }
}

impl fmt::Display for CompatibilityErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Compatibility failure that deliberately does not retain rejected input.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("{code}")]
pub struct CompatibilityError {
    code: CompatibilityErrorCode,
}

impl CompatibilityError {
    /// Stable machine-comparable rejection code.
    #[must_use]
    pub const fn code(self) -> CompatibilityErrorCode {
        self.code
    }
}

/// Selects the exact bundled contract before structural or typed validation.
///
/// Version is checked first, which gives one deterministic failure when both
/// supplied values are unsupported. Same-major versions are not inferred.
pub fn check_event_compatibility(
    schema_version: &str,
    event_type: &str,
) -> Result<(), CompatibilityError> {
    if schema_version != "1.0" {
        return Err(CompatibilityError {
            code: CompatibilityErrorCode::UnsupportedSchemaVersion,
        });
    }
    if !SUPPORTED_EVENT_TYPES.contains(&event_type) {
        return Err(CompatibilityError {
            code: CompatibilityErrorCode::UnsupportedEventType,
        });
    }
    Ok(())
}
