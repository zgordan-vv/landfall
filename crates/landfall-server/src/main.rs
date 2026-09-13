//! Landfall server process entry point.

use std::net::SocketAddr;

use landfall_server::{
    AppState, ObservationWorkerConfig, ObservationWorkerMetrics, auth::hash_token, router,
    run_observation_worker,
};
use landfall_storage::{DatabaseConfig, run_migrations};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("worker") {
        return run_worker().await;
    }
    let bind = std::env::var("LANDFALL_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let address: SocketAddr = bind.parse()?;
    let listener = TcpListener::bind(address).await?;
    let database = DatabaseConfig::from_env()?;
    let pool = database.connect_lazy()?;
    run_migrations(&pool).await?;
    let app = router(AppState {
        ready: true,
        pool: Some(pool),
        bootstrap_token_hash: bootstrap_token_hash()?,
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

fn bootstrap_token_hash() -> Result<Option<[u8; 32]>, std::io::Error> {
    let token = match std::env::var("LANDFALL_BOOTSTRAP_TOKEN_FILE") {
        Ok(path) => Some(std::fs::read_to_string(path)?),
        Err(_) => std::env::var("LANDFALL_BOOTSTRAP_TOKEN").ok(),
    };
    Ok(token.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| hash_token(trimmed))
    }))
}

async fn run_worker() -> Result<(), Box<dyn std::error::Error>> {
    let database = DatabaseConfig::from_env()?;
    let pool = database.connect_lazy()?;
    run_migrations(&pool).await?;
    let cancellation = CancellationToken::new();
    let worker = run_observation_worker(
        pool,
        ObservationWorkerConfig::from_env(),
        cancellation.clone(),
        std::sync::Arc::new(ObservationWorkerMetrics::default()),
    );
    tokio::pin!(worker);
    tokio::select! {
        () = &mut worker => {},
        () = shutdown_signal() => {
            cancellation.cancel();
            worker.await;
        }
    }
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            eprintln!("failed to install Ctrl-C handler: {error}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => eprintln!("failed to install SIGTERM handler: {error}"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
