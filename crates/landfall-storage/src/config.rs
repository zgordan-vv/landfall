//! PostgreSQL connection configuration and pool bootstrap.

use sqlx::{
    PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};

/// Environment-backed database settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL.
    pub database_url: String,
    /// Maximum number of pooled connections.
    pub max_connections: u32,
}

impl DatabaseConfig {
    /// Loads configuration from `DATABASE_URL` and optional `DB_MAX_CONNECTIONS`.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| ConfigError::MissingDatabaseUrl)?;
        let max_connections = std::env::var("DB_MAX_CONNECTIONS")
            .ok()
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| ConfigError::InvalidMaxConnections)?
            .unwrap_or(10);
        if max_connections == 0 {
            return Err(ConfigError::InvalidMaxConnections);
        }
        Ok(Self {
            database_url,
            max_connections,
        })
    }

    /// Creates a lazy pool; no network connection is made by this function.
    pub fn connect_lazy(&self) -> Result<PgPool, sqlx::Error> {
        let options: PgConnectOptions = self.database_url.parse()?;
        Ok(PgPoolOptions::new()
            .max_connections(self.max_connections)
            .connect_lazy_with(options))
    }
}

/// Invalid or incomplete environment configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigError {
    /// `DATABASE_URL` is absent.
    MissingDatabaseUrl,
    /// `DB_MAX_CONNECTIONS` is zero or not an unsigned integer.
    InvalidMaxConnections,
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingDatabaseUrl => f.write_str("DATABASE_URL is required"),
            Self::InvalidMaxConnections => {
                f.write_str("DB_MAX_CONNECTIONS must be a positive integer")
            }
        }
    }
}
impl std::error::Error for ConfigError {}
