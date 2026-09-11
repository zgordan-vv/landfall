//! Immutable in-memory artifact store semantics used by report persistence.

use sha2::{Digest, Sha256};

/// Maximum artifact size accepted by the report path (10 MiB).
pub const MAX_ARTIFACT_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArtifactFormat {
    Json,
    Html,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArtifactPrivacy {
    Internal,
    Shareable,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StoredArtifact {
    pub storage_key: String,
    pub format: ArtifactFormat,
    pub privacy: ArtifactPrivacy,
    pub content_bytes: usize,
    pub sha256: [u8; 32],
    content: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ArtifactError {
    TooLarge,
    EmptyKey,
    ChecksumMismatch,
}

/// Stores immutable bytes and records their content-addressed checksum.
pub fn store_artifact(
    key: impl Into<String>,
    format: ArtifactFormat,
    privacy: ArtifactPrivacy,
    content: Vec<u8>,
) -> Result<StoredArtifact, ArtifactError> {
    let storage_key = key.into();
    if storage_key.trim().is_empty() {
        return Err(ArtifactError::EmptyKey);
    }
    if content.len() > MAX_ARTIFACT_BYTES {
        return Err(ArtifactError::TooLarge);
    }
    let sha256: [u8; 32] = Sha256::digest(&content).into();
    Ok(StoredArtifact {
        storage_key,
        format,
        privacy,
        content_bytes: content.len(),
        sha256,
        content,
    })
}

impl StoredArtifact {
    /// Verifies that supplied bytes still match the recorded checksum.
    pub fn verify(&self, content: &[u8]) -> Result<(), ArtifactError> {
        let digest: [u8; 32] = Sha256::digest(content).into();
        if digest == self.sha256 {
            Ok(())
        } else {
            Err(ArtifactError::ChecksumMismatch)
        }
    }
    /// Returns immutable artifact bytes for download.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn records_size_and_checksum_and_rejects_oversize() {
        let artifact = store_artifact(
            "reports/r1.json",
            ArtifactFormat::Json,
            ArtifactPrivacy::Shareable,
            b"{}".to_vec(),
        )
        .unwrap();
        assert_eq!(artifact.content_bytes, 2);
        assert!(artifact.verify(b"{}").is_ok());
        assert_eq!(
            artifact.verify(b"{\"x\":1}"),
            Err(ArtifactError::ChecksumMismatch)
        );
        assert_eq!(
            store_artifact(
                "r",
                ArtifactFormat::Html,
                ArtifactPrivacy::Internal,
                vec![0; MAX_ARTIFACT_BYTES + 1]
            ),
            Err(ArtifactError::TooLarge)
        );
    }
}
