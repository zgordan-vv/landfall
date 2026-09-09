//! Versioned wire contracts shared by Landfall producers and consumers.
//!
//! JSON Schema under `schemas/events/v1` remains the canonical structural
//! contract. This crate supplies its closed Rust representation and validates
//! scalar and cross-field invariants when values cross a Serde boundary.

pub mod common;
pub mod enums;
pub mod error;
pub mod events;
pub mod values;

pub use common::{EventSource, FingerprintAlgorithm, NormalizedError, SignedBytesFingerprint};
pub use enums::*;
pub use error::{WireValueError, WireValueErrorKind};
pub use events::*;
pub use values::*;

/// Stable namespace for the event protocol wire version `1.0`.
pub mod v1 {
    pub use crate::common::{
        EventSource, FingerprintAlgorithm, NormalizedError, SignedBytesFingerprint,
    };
    pub use crate::enums::*;
    pub use crate::error::{WireValueError, WireValueErrorKind};
    pub use crate::events::*;
    pub use crate::values::*;
}
