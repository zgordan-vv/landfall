//! Identifiers that exist only in the derived domain model.

use std::{fmt, str::FromStr};

use landfall_protocol::{EventId, OperationId};
use uuid::{Uuid, Variant};

/// Failure to parse a canonical `UUIDv7` domain identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DomainIdError {
    field: &'static str,
}

impl DomainIdError {
    /// Returns the field whose identifier was invalid.
    #[must_use]
    pub const fn field(self) -> &'static str {
        self.field
    }
}

impl fmt::Display for DomainIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} must be a canonical lowercase UUIDv7",
            self.field
        )
    }
}

impl std::error::Error for DomainIdError {}

fn parse_uuid_v7(value: &str, field: &'static str) -> Result<Uuid, DomainIdError> {
    let parsed = Uuid::try_parse(value).map_err(|_| DomainIdError { field })?;
    if parsed.hyphenated().to_string() == value {
        validate_uuid_v7(parsed, field)
    } else {
        Err(DomainIdError { field })
    }
}

fn validate_uuid_v7(value: Uuid, field: &'static str) -> Result<Uuid, DomainIdError> {
    if value.get_version_num() == 7 && value.get_variant() == Variant::RFC4122 {
        Ok(value)
    } else {
        Err(DomainIdError { field })
    }
}

macro_rules! domain_uuid_id {
    ($(#[$meta:meta])* $name:ident, $field:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(Uuid);

        impl $name {
            /// Returns the validated UUID value.
            #[must_use]
            pub const fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl FromStr for $name {
            type Err = DomainIdError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                parse_uuid_v7(value, $field).map(Self)
            }
        }

        impl TryFrom<Uuid> for $name {
            type Error = DomainIdError;

            fn try_from(value: Uuid) -> Result<Self, Self::Error> {
                validate_uuid_v7(value, $field).map(Self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(formatter)
            }
        }
    };
}

domain_uuid_id!(
    /// Identity of one derived diagnosis.
    DiagnosticId,
    "diagnostic_id"
);
domain_uuid_id!(
    /// Identity of one advisory recommendation.
    RecommendationId,
    "recommendation_id"
);
domain_uuid_id!(
    /// Identity of a bounded analysis cohort.
    CohortId,
    "cohort_id"
);

macro_rules! semantic_source_id {
    ($(#[$meta:meta])* $name:ident($inner:ty)) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name($inner);

        impl $name {
            /// Creates the narrower domain identity from its source identity.
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            /// Returns the source identity used to create this domain identity.
            #[must_use]
            pub const fn source_id(self) -> $inner {
                self.0
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self::new(value)
            }
        }


        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

semantic_source_id!(
    /// Simulation identity narrowed from a protocol lifecycle operation ID.
    SimulationId(OperationId)
);
semantic_source_id!(
    /// Status-observation identity derived from its immutable source event.
    StatusObservationId(EventId)
);
semantic_source_id!(
    /// Execution-enrichment identity derived from its immutable source event.
    ExecutionMetadataId(EventId)
);
