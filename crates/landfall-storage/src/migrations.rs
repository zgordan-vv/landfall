//! Embedded `SQLx` migrations and schema-version health reporting.

use sqlx::{PgPool, Row};

/// Logical version of the migration set shipped by this binary.
pub const MIGRATION_SET_VERSION: &str = "storage-migrations-v1";

/// The compile-time embedded migration runner.
pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations");

/// Result of comparing the database migration ledger with the embedded set.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MigrationHealth {
    /// Versions successfully recorded in the database ledger.
    pub applied_versions: Vec<i64>,
    /// Versions whose ledger entries are marked unsuccessful.
    pub failed_versions: Vec<i64>,
    /// Versions embedded in this binary.
    pub expected_versions: Vec<i64>,
}

impl MigrationHealth {
    /// Returns whether the database exactly matches the embedded migration set.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.failed_versions.is_empty() && self.applied_versions == self.expected_versions
    }
}

#[must_use]
/// Returns migration versions embedded in this binary, in execution order.
pub fn expected_migration_versions() -> Vec<i64> {
    MIGRATOR.iter().map(|migration| migration.version).collect()
}

/// Applies all pending embedded migrations to the database.
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    MIGRATOR.run(pool).await
}

/// Reads the `SQLx` ledger and compares applied versions with embedded versions.
pub async fn migration_health(pool: &PgPool) -> Result<MigrationHealth, sqlx::Error> {
    let rows = sqlx::query("SELECT version, success FROM _sqlx_migrations ORDER BY version")
        .fetch_all(pool)
        .await?;

    let mut applied_versions = Vec::with_capacity(rows.len());
    let mut failed_versions = Vec::new();
    for row in rows {
        let version: i64 = row.try_get("version")?;
        let success: bool = row.try_get("success")?;
        applied_versions.push(version);
        if !success {
            failed_versions.push(version);
        }
    }

    Ok(MigrationHealth {
        applied_versions,
        failed_versions,
        expected_versions: expected_migration_versions(),
    })
}
