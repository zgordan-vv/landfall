//! Production logging setup with bounded, structured fields.

use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Process role attached to every structured log line.
#[derive(Clone, Copy)]
pub enum ProcessRole {
    /// HTTP API process.
    Server,
    /// Durable observation-worker process.
    ObserverWorker,
}

impl ProcessRole {
    const fn label(self) -> &'static str {
        match self {
            Self::Server => "server",
            Self::ObserverWorker => "observer-worker",
        }
    }
}

/// Installs the stderr JSON subscriber once for the current process.
///
/// `RUST_LOG` controls filtering and defaults to `landfall_server=info`. Log
/// fields are deliberately limited by call sites to identifiers, status codes,
/// bounded counters, and generic error categories — never request bodies,
/// bearer credentials, RPC endpoints, or signatures.
pub fn init_logging(role: ProcessRole) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("landfall_server=info"));
    let subscriber = tracing_subscriber::registry().with(filter).with(
        fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(true)
            .with_target(true)
            .with_writer(std::io::stderr),
    );
    let _ = subscriber.try_init();
    tracing::info!(
        service = "landfall",
        process_role = role.label(),
        "process_started"
    );
}

#[cfg(test)]
mod tests {
    use super::ProcessRole;

    #[test]
    fn roles_have_stable_machine_labels() {
        assert_eq!(ProcessRole::Server.label(), "server");
        assert_eq!(ProcessRole::ObserverWorker.label(), "observer-worker");
    }
}
