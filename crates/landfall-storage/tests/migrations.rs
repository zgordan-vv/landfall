#![allow(missing_docs)]

use landfall_storage::{MIGRATION_SET_VERSION, MigrationHealth, expected_migration_versions};

#[test]
fn embedded_migration_set_is_non_empty_and_ordered() {
    let versions = expected_migration_versions();
    assert!(!versions.is_empty());
    assert!(versions.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(versions.last(), Some(&15));
    assert_eq!(MIGRATION_SET_VERSION, "storage-migrations-v1");
}

#[test]
fn health_is_current_only_when_versions_and_failures_match() {
    let health = MigrationHealth {
        applied_versions: vec![1, 2],
        failed_versions: vec![],
        expected_versions: vec![1, 2],
    };
    assert!(health.is_current());
}
