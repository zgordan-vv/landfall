//! Database configuration tests.

use landfall_storage::DatabaseConfig;

#[tokio::test(flavor = "current_thread")]
async fn lazy_pool_configuration_does_not_require_live_database() {
    let config = DatabaseConfig {
        database_url: "postgres://landfall:landfall@localhost/landfall".into(),
        max_connections: 4,
    };
    assert!(config.connect_lazy().is_ok());
}
