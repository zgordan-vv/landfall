//! Typed errors produced while constructing or decoding protocol values.

use thiserror::Error;

/// A stable category for invalid wire data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WireValueErrorKind {
    /// A string does not match the required lexical form.
    InvalidFormat,
    /// A numeric or collection value is outside its inclusive bounds.
    OutOfRange,
    /// A UUID is valid but is not a canonical `UUIDv7` value.
    InvalidUuidV7,
    /// The event discriminator does not match its attributes type.
    WrongEventType,
    /// Required identity or related evidence is absent.
    MissingRequiredEvidence,
    /// Individually valid fields contradict one another.
    ContradictoryEvidence,
}

/// Failure to construct a strongly typed protocol value.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("invalid {field}: {kind:?}")]
pub struct WireValueError {
    field: &'static str,
    kind: WireValueErrorKind,
}

impl WireValueError {
    /// Creates an error without retaining or reflecting the rejected value.
    #[must_use]
    pub const fn new(field: &'static str, kind: WireValueErrorKind) -> Self {
        Self { field, kind }
    }

    /// Returns the safe field/category label.
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.field
    }

    /// Returns the machine-comparable error kind.
    #[must_use]
    pub const fn kind(&self) -> WireValueErrorKind {
        self.kind
    }
}
