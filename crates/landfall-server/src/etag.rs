//! Conditional-read semantics tied to monotonic projection versions.

use serde::Serialize;

/// Metadata returned with a projection-backed response.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectionVersion {
    pub projection_version: u64,
    pub etag: String,
}

/// Result of evaluating an `If-None-Match` request header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalRead {
    NotModified,
    Modified,
}

/// Creates a strong, quoted ETag whose only input is the projection version.
#[must_use]
pub fn etag_for(projection_version: u64) -> String {
    format!("\"projection-{projection_version}\"")
}

/// Builds response metadata for a projection version.
#[must_use]
pub fn projection_version(projection_version: u64) -> ProjectionVersion {
    ProjectionVersion {
        projection_version,
        etag: etag_for(projection_version),
    }
}

/// Returns `NotModified` only for an exact current ETag match.
#[must_use]
pub fn conditional_read(projection_version: u64, if_none_match: Option<&str>) -> ConditionalRead {
    if if_none_match.is_some_and(|candidate| candidate.trim() == etag_for(projection_version)) {
        ConditionalRead::NotModified
    } else {
        ConditionalRead::Modified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_is_stable_for_version_and_changes_when_version_advances() {
        assert_eq!(etag_for(42), "\"projection-42\"");
        assert_eq!(etag_for(42), etag_for(42));
        assert_ne!(etag_for(42), etag_for(43));
        assert_eq!(
            conditional_read(42, Some("\"projection-42\"")),
            ConditionalRead::NotModified
        );
        assert_eq!(
            conditional_read(43, Some("\"projection-42\"")),
            ConditionalRead::Modified
        );
    }
}
