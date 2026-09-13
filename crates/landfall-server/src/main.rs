//! Landfall server process entry point.

use std::net::SocketAddr;

use landfall_observer::{
    JsonRpcClient, ReqwestRouteClient, execution_enriched_event, get_transaction,
    normalize_execution, normalize_signature_status, status_observed_event,
};
use landfall_server::{AppState, refresh_trace_projection, router};
use landfall_storage::{
    DatabaseConfig, IngestEvent, claim_observation_job, complete_observation_job,
    ensure_raw_event_partition, ingest_atomically, load_observation_target,
    reclaim_expired_observation_jobs, retry_observation_job, run_migrations,
};
use sha2::{Digest, Sha256};
use sqlx::Row;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind = std::env::var("LANDFALL_BIND").unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let address: SocketAddr = bind.parse()?;
    let listener = TcpListener::bind(address).await?;
    let database = DatabaseConfig::from_env()?;
    let pool = database.connect_lazy()?;
    run_migrations(&pool).await?;
    let recovery_pool = pool.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if let Err(error) = reclaim_expired_observation_jobs(&recovery_pool).await {
                eprintln!("observation lease recovery failed: {error}");
            }
        }
    });
    let observer_pool = pool.clone();
    tokio::spawn(async move {
        loop {
            match claim_observation_job(&observer_pool, "landfall-server", 60).await {
                Ok(Some(job)) => {
                    let result = async {
                        let target = load_observation_target(&observer_pool, job.trace_id).await.map_err(|_| "observation target lookup failed")?.ok_or("no enabled route or submission signature")?;
                        let client = ReqwestRouteClient::new(target.route_id.to_string(), target.endpoint.clone(), std::time::Duration::from_secs(10)).map_err(|_| "rpc client setup failed")?;
                        let rpc = JsonRpcClient::new(target.endpoint, client);
                        let status = rpc.get_signature_status(&target.signature).await.map_err(|_| "rpc status query failed")?;
                        let observed = normalize_signature_status(status);
                        let row = sqlx::query("SELECT project_id, environment_id FROM reporting.traces WHERE trace_id = $1").bind(job.trace_id).fetch_one(&observer_pool).await.map_err(|_| "trace lookup failed")?;
                        let project_id: uuid::Uuid = row.get("project_id"); let environment_id: uuid::Uuid = row.get("environment_id");
                        let now = time::OffsetDateTime::now_utc(); let occurred = now.format(&time::format_description::well_known::Rfc3339).map_err(|_| "time format failed")?;
                        let payload = status_observed_event(&project_id.to_string(), &environment_id.to_string(), &job.trace_id.to_string(), &target.route_id.to_string(), &uuid::Uuid::now_v7().to_string(), &occurred, &target.signature, &observed);
                        let event_id = payload["event_id"].as_str().and_then(|v| uuid::Uuid::parse_str(v).ok()).ok_or("event id failed")?;
                        ensure_raw_event_partition(&observer_pool, now.date()).await.map_err(|_| "partition failed")?;
                        let mut events = vec![IngestEvent { event_id, project_id, environment_id, trace_id: Some(job.trace_id), event_type: "solana.status.observed".into(), occurred_at: now, payload: payload.clone(), payload_hash: Sha256::digest(serde_json::to_vec(&payload).map_err(|_| "payload encode failed")?).to_vec() }];
                        if observed.source_result == "found" {
                            if let Some(transaction) = get_transaction(&rpc, &target.signature).await.map_err(|_| "rpc transaction query failed")? {
                                let execution = normalize_execution(&transaction).map_err(|_| "rpc transaction response malformed")?;
                                let block_time = execution.block_time.and_then(|seconds| time::OffsetDateTime::from_unix_timestamp(seconds).ok()).and_then(|value| value.format(&time::format_description::well_known::Rfc3339).ok());
                                let payload = execution_enriched_event(&project_id.to_string(), &environment_id.to_string(), &job.trace_id.to_string(), &target.route_id.to_string(), &uuid::Uuid::now_v7().to_string(), &occurred, &target.signature, observed.commitment.as_deref().unwrap_or("processed"), &execution, block_time.as_deref());
                                let event_id = payload["event_id"].as_str().and_then(|v| uuid::Uuid::parse_str(v).ok()).ok_or("execution event id failed")?;
                                events.push(IngestEvent { event_id, project_id, environment_id, trace_id: Some(job.trace_id), event_type: "solana.execution.enriched".into(), occurred_at: now, payload: payload.clone(), payload_hash: Sha256::digest(serde_json::to_vec(&payload).map_err(|_| "execution payload encode failed")?).to_vec() });
                            }
                        }
                        ingest_atomically(&observer_pool, uuid::Uuid::now_v7(), &events).await.map_err(|_| "event ingest failed")?;
                        refresh_trace_projection(&observer_pool, project_id, environment_id, job.trace_id).await.map_err(|_| "projection failed")?;
                        Ok::<(), &'static str>(())
                    }.await;
                    match result {
                        Ok(()) => {
                            let _ = complete_observation_job(&observer_pool, job.job_id).await;
                        }
                        Err(error) => {
                            let _ =
                                retry_observation_job(&observer_pool, job.job_id, error, 5).await;
                        }
                    }
                }
                Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
                Err(_) => tokio::time::sleep(std::time::Duration::from_secs(1)).await,
            }
        }
    });
    let app = router(AppState {
        ready: true,
        pool: Some(pool),
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
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
