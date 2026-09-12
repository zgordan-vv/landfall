//! Backup manifest and restore validation primitives.

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BackupManifest {
    pub backup_id: String,
    pub schema_version: String,
    pub bytes: u64,
    pub sha256: [u8; 32],
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RestoreError {
    EmptyId,
    SchemaMismatch,
    SizeMismatch,
    ChecksumMismatch,
}

/// Creates a manifest for an immutable database backup payload.
pub fn create_manifest(
    backup_id: impl Into<String>,
    schema_version: impl Into<String>,
    payload: &[u8],
) -> Result<BackupManifest, RestoreError> {
    let backup_id = backup_id.into();
    let schema_version = schema_version.into();
    if backup_id.trim().is_empty() || schema_version.trim().is_empty() {
        return Err(RestoreError::EmptyId);
    }
    Ok(BackupManifest {
        backup_id,
        schema_version,
        bytes: payload.len() as u64,
        sha256: Sha256::digest(payload).into(),
    })
}

/// Validates a restore payload before it can replace a local fixture database.
pub fn validate_restore(
    manifest: &BackupManifest,
    expected_schema: &str,
    payload: &[u8],
) -> Result<(), RestoreError> {
    if manifest.schema_version != expected_schema {
        return Err(RestoreError::SchemaMismatch);
    }
    if manifest.bytes != payload.len() as u64 {
        return Err(RestoreError::SizeMismatch);
    }
    let digest: [u8; 32] = Sha256::digest(payload).into();
    if manifest.sha256 != digest {
        return Err(RestoreError::ChecksumMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restore_requires_matching_schema_size_and_checksum() {
        let payload = b"fixture backup";
        let manifest = create_manifest("backup-1", "schema-v1", payload).unwrap();
        assert!(validate_restore(&manifest, "schema-v1", payload).is_ok());
        assert_eq!(
            validate_restore(&manifest, "schema-v2", payload),
            Err(RestoreError::SchemaMismatch)
        );
        assert_eq!(
            validate_restore(&manifest, "schema-v1", b"changed"),
            Err(RestoreError::SizeMismatch)
        );
    }
}
