//! Reusable structured values embedded by multiple event types.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::{
    enums::{NormalizedErrorCategory, SourceKind},
    error::{WireValueError, WireValueErrorKind},
    values::{
        AppVersion, BoundedText, ComponentName, ErrorCode, FingerprintHex, FingerprintKeyId,
        SourceInstanceId, SourceVersion,
    },
};

/// Identity of the component that directly observed and emitted an event.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EventSource {
    /// Producer class.
    pub kind: SourceKind,
    /// Bounded producer component name.
    pub name: ComponentName,
    /// Bounded producer component version.
    pub version: SourceVersion,
    /// Identity of one process/runtime instance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<SourceInstanceId>,
    /// Optional low-cardinality customer service label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<ComponentName>,
    /// Optional deployment/build label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_version: Option<AppVersion>,
}

/// Normalized error plus bounded and redacted original evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedError {
    /// Stable analytical error category.
    pub category: NormalizedErrorCategory,
    /// Optional bounded original error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<ErrorCode>,
    /// Optional bounded, redacted original message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<BoundedText>,
    /// Optional failing instruction index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_index: Option<u8>,
    /// Optional program-specific unsigned error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_code: Option<u32>,
}

/// Fixed fingerprint algorithm identifier for protocol v1.0.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FingerprintAlgorithm;

impl FingerprintAlgorithm {
    /// Canonical serialized algorithm identifier.
    pub const VALUE: &'static str = "lf-hmac-sha256-v1";
}

impl Default for FingerprintAlgorithm {
    fn default() -> Self {
        Self
    }
}

impl Serialize for FingerprintAlgorithm {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(Self::VALUE)
    }
}

impl<'de> Deserialize<'de> for FingerprintAlgorithm {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value == Self::VALUE {
            Ok(Self)
        } else {
            Err(de::Error::custom(WireValueError::new(
                "fingerprint.algorithm",
                WireValueErrorKind::InvalidFormat,
            )))
        }
    }
}

/// HMAC-SHA256 fingerprint of exact serialized signed transaction bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignedBytesFingerprint {
    /// Fixed protocol v1 fingerprint algorithm.
    pub algorithm: FingerprintAlgorithm,
    /// Identity of the environment fingerprint key, never the key itself.
    pub key_id: FingerprintKeyId,
    /// Lowercase hexadecimal HMAC output.
    pub value_hex: FingerprintHex,
}
