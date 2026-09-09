//! Validated scalar wire values and semantically distinct identifiers.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};
use uuid::{Uuid, Variant};

use crate::error::{WireValueError, WireValueErrorKind};

macro_rules! uuid_id {
    ($(#[$meta:meta])* $name:ident, $field:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(Uuid);

        impl $name {
            /// Returns the parsed UUID value.
            #[must_use]
            pub const fn into_uuid(self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = WireValueError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                parse_uuid_v7(value, $field).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(de::Error::custom)
            }
        }
    };
}

fn parse_uuid_v7(value: &str, field: &'static str) -> Result<Uuid, WireValueError> {
    let parsed = Uuid::try_parse(value)
        .map_err(|_| WireValueError::new(field, WireValueErrorKind::InvalidUuidV7))?;
    if parsed.get_version_num() != 7
        || parsed.get_variant() != Variant::RFC4122
        || parsed.hyphenated().to_string() != value
    {
        return Err(WireValueError::new(
            field,
            WireValueErrorKind::InvalidUuidV7,
        ));
    }
    Ok(parsed)
}

uuid_id!(
    /// Unique identity of an immutable event.
    EventId,
    "event_id"
);
uuid_id!(
    /// Owning Landfall project identity.
    ProjectId,
    "project_id"
);
uuid_id!(
    /// Owning environment and cluster boundary.
    EnvironmentId,
    "environment_id"
);
uuid_id!(
    /// Identity of one transaction lifecycle trace.
    TraceId,
    "trace_id"
);
uuid_id!(
    /// Identity of an explicit customer business action.
    BusinessActionId,
    "business_action_id"
);
uuid_id!(
    /// Identity of one real submission attempt.
    AttemptId,
    "attempt_id"
);
uuid_id!(
    /// Identity of a bounded lifecycle operation such as signing or waiting.
    OperationId,
    "operation_id"
);
uuid_id!(
    /// Identity of one producer process/runtime instance.
    SourceInstanceId,
    "source.instance_id"
);
uuid_id!(
    /// Identity of a configured fingerprint key, never the secret key itself.
    FingerprintKeyId,
    "fingerprint.key_id"
);
uuid_id!(
    /// Identity of a configured RPC submission route.
    RouteId,
    "route_id"
);
uuid_id!(
    /// Identity of a configured independent observer source.
    ObserverSourceId,
    "observer_source_id"
);
uuid_id!(
    /// Identity of one ingestion transport batch.
    BatchId,
    "batch_id"
);

macro_rules! decimal_u64 {
    ($(#[$meta:meta])* $name:ident, $field:literal) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(u64);

        impl $name {
            /// Creates the exact value from its runtime integer representation.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the runtime integer representation.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl FromStr for $name {
            type Err = WireValueError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                parse_u64_decimal(value, $field).map(Self)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.collect_str(self)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                value.parse().map_err(de::Error::custom)
            }
        }
    };
}

fn parse_u64_decimal(value: &str, field: &'static str) -> Result<u64, WireValueError> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(WireValueError::new(
            field,
            WireValueErrorKind::InvalidFormat,
        ));
    }
    value
        .parse()
        .map_err(|_| WireValueError::new(field, WireValueErrorKind::OutOfRange))
}

decimal_u64!(
    /// Non-negative duration or monotonic-clock value in nanoseconds.
    DurationNs,
    "duration_ns"
);
decimal_u64!(
    /// Solana slot encoded as a canonical decimal string.
    Slot,
    "slot"
);
decimal_u64!(
    /// Solana block height encoded as a canonical decimal string.
    BlockHeight,
    "block_height"
);
decimal_u64!(
    /// Lamports encoded as a canonical decimal string.
    Lamports,
    "lamports"
);
decimal_u64!(
    /// Compute units encoded as a canonical decimal string.
    ComputeUnits,
    "compute_units"
);
decimal_u64!(
    /// Non-negative confirmation count encoded as a canonical decimal string.
    ConfirmationCount,
    "confirmation_count"
);

/// Signed 64-bit integer serialized as a canonical decimal string.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Int64Decimal(i64);

impl Int64Decimal {
    /// Creates the exact value from its runtime integer representation.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Returns the runtime integer representation.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Int64Decimal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for Int64Decimal {
    type Err = WireValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let digits = value.strip_prefix('-').unwrap_or(value);
        if value.is_empty()
            || value.starts_with('+')
            || value == "-0"
            || (value.starts_with('0') && value.len() > 1)
            || (value.starts_with("-0") && value.len() > 2)
            || digits.is_empty()
            || !digits.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(WireValueError::new(
                "int64_decimal",
                WireValueErrorKind::InvalidFormat,
            ));
        }
        value
            .parse()
            .map(Self)
            .map_err(|_| WireValueError::new("int64_decimal", WireValueErrorKind::OutOfRange))
    }
}

impl Serialize for Int64Decimal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Int64Decimal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}

macro_rules! bounded_integer {
    ($(#[$meta:meta])* $name:ident($inner:ty), $field:literal, $minimum:expr, $maximum:expr) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name($inner);

        impl $name {
            /// Returns the validated integer.
            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl TryFrom<$inner> for $name {
            type Error = WireValueError;

            fn try_from(value: $inner) -> Result<Self, Self::Error> {
                if ($minimum..=$maximum).contains(&value) {
                    Ok(Self(value))
                } else {
                    Err(WireValueError::new($field, WireValueErrorKind::OutOfRange))
                }
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                <$inner>::deserialize(deserializer)?.try_into().map_err(de::Error::custom)
            }
        }
    };
}

bounded_integer!(
    /// Number of signatures required by a transaction, in `1..=64`.
    RequiredSignatures(u8),
    "required_signatures",
    1,
    64
);
bounded_integer!(
    /// One-based submission or retry sequence, in `1..=10_000`.
    AttemptSequence(u16),
    "attempt_sequence",
    1,
    10_000
);
bounded_integer!(
    /// Maximum RPC retry count, in `0..=1_000`.
    MaxRetries(u16),
    "max_retries",
    0,
    1_000
);

macro_rules! string_value {
    ($(#[$meta:meta])* $name:ident, $field:literal, $validator:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Returns the validated wire string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl TryFrom<String> for $name {
            type Error = WireValueError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if $validator(&value) {
                    Ok(Self(value))
                } else {
                    Err(WireValueError::new($field, WireValueErrorKind::InvalidFormat))
                }
            }
        }

        impl FromStr for $name {
            type Err = WireValueError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                value.to_owned().try_into()
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                String::deserialize(deserializer)?.try_into().map_err(de::Error::custom)
            }
        }
    };
}

fn length_between(value: &str, minimum: usize, maximum: usize) -> bool {
    let length = value.chars().count();
    (minimum..=maximum).contains(&length)
}

fn valid_token(value: &str) -> bool {
    if !value.is_ascii() || value.len() > 64 {
        return false;
    }
    let mut segments = value.split(['.', '_', '-']);
    let Some(first) = segments.next() else {
        return false;
    };
    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    };
    first.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && valid_segment(first)
        && segments.all(valid_segment)
}

fn valid_component_name(value: &str) -> bool {
    value.is_ascii()
        && length_between(value, 1, 64)
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn valid_protocol_version(value: &str) -> bool {
    if !value.is_ascii() || value.len() > 19 {
        return false;
    }
    let Some((major, minor)) = value.split_once('.') else {
        return false;
    };
    let valid_part = |part: &str| {
        !part.is_empty()
            && part.len() <= 9
            && part.bytes().all(|byte| byte.is_ascii_digit())
            && (part == "0" || !part.starts_with('0'))
    };
    valid_part(major) && valid_part(minor) && !minor.contains('.')
}

fn valid_label(value: &str) -> bool {
    length_between(value, 1, 128)
}

fn valid_bounded_text(value: &str) -> bool {
    length_between(value, 1, 512)
}

fn valid_source_version(value: &str) -> bool {
    length_between(value, 1, 64)
}

fn valid_app_version(value: &str) -> bool {
    length_between(value, 1, 128)
}

fn is_base58(byte: u8) -> bool {
    matches!(byte, b'1'..=b'9' | b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | b'a'..=b'k' | b'm'..=b'z')
}

fn valid_signature(value: &str) -> bool {
    matches!(value.len(), 87 | 88) && value.bytes().all(is_base58)
}

fn valid_blockhash(value: &str) -> bool {
    (32..=44).contains(&value.len()) && value.bytes().all(is_base58)
}

fn valid_hex_fingerprint(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_json_pointer(value: &str) -> bool {
    if value.chars().count() > 256 || (!value.is_empty() && !value.starts_with('/')) {
        return false;
    }
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}

string_value!(
    /// Customer-defined bounded token such as a flow or reason.
    Token,
    "token",
    valid_token
);
string_value!(
    /// Human label bounded to 128 Unicode scalar values.
    Label,
    "label",
    valid_label
);
string_value!(
    /// Redacted free text bounded to 512 Unicode scalar values.
    BoundedText,
    "bounded_text",
    valid_bounded_text
);
string_value!(
    /// Source or service name using the protocol's low-cardinality alphabet.
    ComponentName,
    "component_name",
    valid_component_name
);
string_value!(
    /// Bounded producer component version.
    SourceVersion,
    "source_version",
    valid_source_version
);
string_value!(
    /// Bounded application deployment/build version.
    AppVersion,
    "app_version",
    valid_app_version
);
string_value!(
    /// Canonical `MAJOR.MINOR` policy or wire version.
    ProtocolVersion,
    "protocol_version",
    valid_protocol_version
);
string_value!(
    /// Base58 Solana transaction signature.
    SolanaSignature,
    "signature",
    valid_signature
);
string_value!(
    /// Base58 Solana recent blockhash.
    SolanaBlockhash,
    "blockhash",
    valid_blockhash
);
string_value!(
    /// Lowercase hexadecimal HMAC-SHA256 output.
    FingerprintHex,
    "value_hex",
    valid_hex_fingerprint
);
string_value!(
    /// RFC 6901 JSON Pointer bounded to 256 characters.
    JsonPointer,
    "json_pointer",
    valid_json_pointer
);
string_value!(
    /// Bounded original error code.
    ErrorCode,
    "error_code",
    valid_source_version
);
string_value!(
    /// Bounded opaque receipt returned by a submission route.
    RouteReceipt,
    "route_receipt",
    valid_label
);

/// The only event wire version implemented by this module.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SchemaVersion;

impl SchemaVersion {
    /// Canonical serialized value.
    pub const VALUE: &'static str = "1.0";
}

impl Default for SchemaVersion {
    fn default() -> Self {
        Self
    }
}

impl Serialize for SchemaVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(Self::VALUE)
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == Self::VALUE {
            Ok(Self)
        } else {
            Err(de::Error::custom(WireValueError::new(
                "schema_version",
                WireValueErrorKind::InvalidFormat,
            )))
        }
    }
}

/// UTC RFC 3339 timestamp with at most nanosecond precision.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct UtcTimestamp(OffsetDateTime);

impl UtcTimestamp {
    /// Creates a UTC-normalized timestamp.
    #[must_use]
    pub fn new(value: OffsetDateTime) -> Self {
        Self(value.to_offset(UtcOffset::UTC))
    }

    /// Returns the parsed timestamp.
    #[must_use]
    pub const fn get(self) -> OffsetDateTime {
        self.0
    }
}

impl fmt::Display for UtcTimestamp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = self.0.format(&Rfc3339).map_err(|_| fmt::Error)?;
        formatter.write_str(&rendered)
    }
}

impl FromStr for UtcTimestamp {
    type Err = WireValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if !valid_utc_timestamp_shape(value) {
            return Err(WireValueError::new(
                "timestamp",
                WireValueErrorKind::InvalidFormat,
            ));
        }
        OffsetDateTime::parse(value, &Rfc3339)
            .map(Self)
            .map_err(|_| WireValueError::new("timestamp", WireValueErrorKind::InvalidFormat))
    }
}

fn valid_utc_timestamp_shape(value: &str) -> bool {
    if !value.is_ascii() || !(20..=30).contains(&value.len()) || !value.ends_with('Z') {
        return false;
    }
    let bytes = value.as_bytes();
    if bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return false;
    }
    let fraction = &value[19..value.len() - 1];
    fraction.is_empty()
        || (fraction.starts_with('.')
            && (1..=9).contains(&(fraction.len() - 1))
            && fraction[1..].bytes().all(|byte| byte.is_ascii_digit()))
}

impl Serialize for UtcTimestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for UtcTimestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(de::Error::custom)
    }
}
