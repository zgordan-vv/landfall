//! Durable Solana observation worker runtime.

use std::{sync::Arc, time::Duration};

use crate::secrets::RouteSecretCipher;
use landfall_observer::{
    JsonRpcClient, ReqwestRouteClient, execution_enriched_event, get_transaction,
    normalize_execution, normalize_signature_status, status_observed_event,
};
use landfall_storage::{
    IngestEvent, claim_observation_job, complete_observation_job, ensure_raw_event_partition,
    ingest_atomically, load_observation_target, reclaim_expired_observation_jobs,
    retry_observation_job,
};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

/// Runtime settings for the observation worker. Every value has a safe default
/// and can be overridden with a `LANDFALL_OBSERVER_*` environment variable.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ObservationWorkerConfig {
    pub concurrency: usize,
    pub poll_interval: Duration,
    pub retry_delay: Duration,
    pub lease_duration: Duration,
    pub recovery_interval: Duration,
}

impl ObservationWorkerConfig {
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            concurrency: read_positive_usize("LANDFALL_OBSERVER_CONCURRENCY", 4),
            poll_interval: Duration::from_millis(read_positive_u64(
                "LANDFALL_OBSERVER_POLL_INTERVAL_MS",
                500,
            )),
            retry_delay: Duration::from_secs(read_positive_u64(
                "LANDFALL_OBSERVER_RETRY_DELAY_SECONDS",
                5,
            )),
            lease_duration: Duration::from_secs(read_positive_u64(
                "LANDFALL_OBSERVER_LEASE_SECONDS",
                60,
            )),
            recovery_interval: Duration::from_secs(read_positive_u64(
                "LANDFALL_OBSERVER_RECOVERY_INTERVAL_SECONDS",
                30,
            )),
        }
    }
}

impl Default for ObservationWorkerConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            poll_interval: Duration::from_millis(500),
            retry_delay: Duration::from_secs(5),
            lease_duration: Duration::from_secs(60),
            recovery_interval: Duration::from_secs(30),
        }
    }
}

/// Counters owned by one worker process. They are intentionally in-process:
/// durable job state remains the source of truth in PostgreSQL.
#[derive(Default)]
pub struct ObservationWorkerMetrics {
    claimed_total: std::sync::atomic::AtomicU64,
    completed_total: std::sync::atomic::AtomicU64,
    retried_total: std::sync::atomic::AtomicU64,
    dead_lettered_total: std::sync::atomic::AtomicU64,
    failed_claims_total: std::sync::atomic::AtomicU64,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ObservationWorkerMetricsSnapshot {
    pub claimed_total: u64,
    pub completed_total: u64,
    pub retried_total: u64,
    pub dead_lettered_total: u64,
    pub failed_claims_total: u64,
}

impl ObservationWorkerMetrics {
    #[must_use]
    pub fn snapshot(&self) -> ObservationWorkerMetricsSnapshot {
        use std::sync::atomic::Ordering;
        ObservationWorkerMetricsSnapshot {
            claimed_total: self.claimed_total.load(Ordering::Relaxed),
            completed_total: self.completed_total.load(Ordering::Relaxed),
            retried_total: self.retried_total.load(Ordering::Relaxed),
            dead_lettered_total: self.dead_lettered_total.load(Ordering::Relaxed),
            failed_claims_total: self.failed_claims_total.load(Ordering::Relaxed),
        }
    }

    fn claimed(&self) {
        self.claimed_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    fn completed(&self) {
        self.completed_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    fn retried(&self) {
        self.retried_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    fn dead_lettered(&self) {
        self.dead_lettered_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    fn failed_claim(&self) {
        self.failed_claims_total
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Runs durable observation polling until cancelled. `concurrency` independent
/// loops use `FOR UPDATE SKIP LOCKED`, so each claimed job has one owner.
pub async fn run_observation_worker(
    pool: PgPool,
    config: ObservationWorkerConfig,
    cancellation: CancellationToken,
    metrics: Arc<ObservationWorkerMetrics>,
    route_secret_cipher: Option<Arc<RouteSecretCipher>>,
) {
    let _ = reclaim_expired_observation_jobs(&pool).await;
    let mut tasks = JoinSet::new();

    for index in 0..config.concurrency {
        tasks.spawn(run_poll_loop(
            pool.clone(),
            format!("landfall-observer-{index}"),
            config,
            cancellation.child_token(),
            Arc::clone(&metrics),
            route_secret_cipher.clone(),
        ));
    }
    tasks.spawn(run_recovery_loop(
        pool,
        config.recovery_interval,
        cancellation.child_token(),
    ));

    while tasks.join_next().await.is_some() {}
}

async fn run_recovery_loop(pool: PgPool, interval: Duration, cancellation: CancellationToken) {
    loop {
        tokio::select! {
            () = cancellation.cancelled() => return,
            () = tokio::time::sleep(interval) => {
                if let Err(error) = reclaim_expired_observation_jobs(&pool).await {
                    tracing::warn!(%error, "observation lease recovery failed");
                }
            }
        }
    }
}

async fn run_poll_loop(
    pool: PgPool,
    worker_id: String,
    config: ObservationWorkerConfig,
    cancellation: CancellationToken,
    metrics: Arc<ObservationWorkerMetrics>,
    route_secret_cipher: Option<Arc<RouteSecretCipher>>,
) {
    loop {
        tokio::select! {
            () = cancellation.cancelled() => return,
            claimed = claim_observation_job(&pool, &worker_id, config.lease_duration.as_secs() as i64) => {
                match claimed {
                    Ok(Some(job)) => {
                        metrics.claimed();
                        match observe_job(&pool, job.trace_id, job.job_id, route_secret_cipher.as_deref()).await {
                            Ok(()) => match complete_observation_job(&pool, job.job_id).await {
                                Ok(()) => metrics.completed(),
                                Err(error) => tracing::error!(%error, job_id = %job.job_id, "could not complete observation job"),
                            },
                            Err(error) => {
                                let retry_delay = retry_delay(config.retry_delay, job.attempts);
                                match retry_observation_job(&pool, job.job_id, &error, retry_delay.as_secs() as i64).await {
                                    Ok(landfall_storage::ObservationRetryOutcome::RetryScheduled) => {
                                        metrics.retried();
                                        tracing::warn!(job_id = %job.job_id, attempt = job.attempts, retry_seconds = retry_delay.as_secs(), %error, "observation job retry scheduled");
                                    }
                                    Ok(landfall_storage::ObservationRetryOutcome::DeadLettered) => {
                                        metrics.dead_lettered();
                                        tracing::error!(job_id = %job.job_id, attempt = job.attempts, %error, "observation job dead-lettered");
                                    }
                                    Err(retry_error) => tracing::error!(%retry_error, job_id = %job.job_id, "could not retry observation job"),
                                }
                            }
                        }
                    }
                    Ok(None) => wait_or_cancel(config.poll_interval, &cancellation).await,
                    Err(error) => {
                        metrics.failed_claim();
                        tracing::warn!(%error, "observation job claim failed");
                        wait_or_cancel(config.poll_interval, &cancellation).await;
                    }
                }
            }
        }
    }
}

/// Exponential retry delay with a five-minute ceiling. The database owns the
/// next attempt timestamp; this function only supplies the bounded interval.
#[must_use]
pub fn retry_delay(base: Duration, attempt: i32) -> Duration {
    let exponent = u32::try_from(attempt.saturating_sub(1))
        .unwrap_or(0)
        .min(16);
    base.saturating_mul(2_u32.saturating_pow(exponent))
        .min(Duration::from_secs(300))
}

async fn wait_or_cancel(duration: Duration, cancellation: &CancellationToken) {
    tokio::select! {
        () = cancellation.cancelled() => {},
        () = tokio::time::sleep(duration) => {},
    }
}

async fn observe_job(
    pool: &PgPool,
    trace_id: uuid::Uuid,
    job_id: uuid::Uuid,
    route_secret_cipher: Option<&RouteSecretCipher>,
) -> Result<(), String> {
    let target = load_observation_target(pool, trace_id)
        .await
        .map_err(|_| "observation target lookup failed".to_owned())?
        .ok_or_else(|| "no enabled route or submission signature".to_owned())?;
    let endpoint = match (
        &target.endpoint_ciphertext,
        &target.endpoint_nonce,
        route_secret_cipher,
    ) {
        (Some(ciphertext), Some(nonce), Some(cipher)) => cipher
            .decrypt(ciphertext, nonce)
            .map_err(|_| "rpc endpoint decryption failed".to_owned())?,
        (None, None, _) => target
            .endpoint
            .ok_or_else(|| "configured route has no endpoint".to_owned())?,
        _ => return Err("encrypted RPC route requires route secret key".to_owned()),
    };
    let client = ReqwestRouteClient::new(
        target.route_id.to_string(),
        endpoint.clone(),
        Duration::from_secs(10),
    )
    .map_err(|_| "rpc client setup failed".to_owned())?;
    let rpc = JsonRpcClient::new(endpoint, client);
    let status = rpc
        .get_signature_status(&target.signature)
        .await
        .map_err(|_| "rpc status query failed".to_owned())?;
    let observed = normalize_signature_status(status);
    let row =
        sqlx::query("SELECT project_id, environment_id FROM reporting.traces WHERE trace_id = $1")
            .bind(trace_id)
            .fetch_one(pool)
            .await
            .map_err(|_| "trace lookup failed".to_owned())?;
    let project_id: uuid::Uuid = row.get("project_id");
    let environment_id: uuid::Uuid = row.get("environment_id");
    let now = time::OffsetDateTime::now_utc();
    let occurred = now
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|_| "time format failed".to_owned())?;
    let payload = status_observed_event(
        &project_id.to_string(),
        &environment_id.to_string(),
        &trace_id.to_string(),
        &target.route_id.to_string(),
        &uuid::Uuid::now_v7().to_string(),
        &occurred,
        &target.signature,
        &observed,
    );
    let status_event_id = event_id(&payload, "event id failed")?;
    ensure_raw_event_partition(pool, now.date())
        .await
        .map_err(|_| "partition failed".to_owned())?;
    let mut events = vec![ingest_event(
        status_event_id,
        project_id,
        environment_id,
        trace_id,
        "solana.status.observed",
        now,
        payload,
    )?];
    if observed.source_result == "found" {
        if let Some(transaction) = get_transaction(&rpc, &target.signature)
            .await
            .map_err(|_| "rpc transaction query failed".to_owned())?
        {
            let execution = normalize_execution(&transaction)
                .map_err(|_| "rpc transaction response malformed".to_owned())?;
            let block_time = execution
                .block_time
                .and_then(|seconds| time::OffsetDateTime::from_unix_timestamp(seconds).ok())
                .and_then(|value| {
                    value
                        .format(&time::format_description::well_known::Rfc3339)
                        .ok()
                });
            let payload = execution_enriched_event(
                &project_id.to_string(),
                &environment_id.to_string(),
                &trace_id.to_string(),
                &target.route_id.to_string(),
                &uuid::Uuid::now_v7().to_string(),
                &occurred,
                &target.signature,
                observed.commitment.as_deref().unwrap_or("processed"),
                &execution,
                block_time.as_deref(),
            );
            let execution_event_id = event_id(&payload, "execution event id failed")?;
            events.push(ingest_event(
                execution_event_id,
                project_id,
                environment_id,
                trace_id,
                "solana.execution.enriched",
                now,
                payload,
            )?);
        }
    }
    ingest_atomically(pool, uuid::Uuid::now_v7(), &events)
        .await
        .map_err(|_| "event ingest failed".to_owned())?;
    crate::refresh_trace_projection(pool, project_id, environment_id, trace_id)
        .await
        .map_err(|_| "projection failed".to_owned())?;
    tracing::debug!(%job_id, %trace_id, "observation job completed");
    Ok(())
}

fn event_id(payload: &serde_json::Value, error: &str) -> Result<uuid::Uuid, String> {
    payload["event_id"]
        .as_str()
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .ok_or_else(|| error.to_owned())
}

fn ingest_event(
    event_id: uuid::Uuid,
    project_id: uuid::Uuid,
    environment_id: uuid::Uuid,
    trace_id: uuid::Uuid,
    event_type: &str,
    occurred_at: time::OffsetDateTime,
    payload: serde_json::Value,
) -> Result<IngestEvent, String> {
    let payload_hash = Sha256::digest(
        serde_json::to_vec(&payload).map_err(|_| "payload encode failed".to_owned())?,
    )
    .to_vec();
    Ok(IngestEvent {
        event_id,
        project_id,
        environment_id,
        trace_id: Some(trace_id),
        event_type: event_type.to_owned(),
        occurred_at,
        payload,
        payload_hash,
    })
}

fn read_positive_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &usize| *value > 0)
        .unwrap_or(default)
}

fn read_positive_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &u64| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::{ObservationWorkerConfig, ObservationWorkerMetrics, retry_delay};
    use std::time::Duration;

    #[test]
    fn defaults_bound_polling_and_concurrency() {
        let config = ObservationWorkerConfig::default();
        assert_eq!(config.concurrency, 4);
        assert!(config.poll_interval.as_millis() > 0);
        assert!(config.lease_duration > config.poll_interval);
    }

    #[test]
    fn metrics_start_at_zero() {
        let metrics = ObservationWorkerMetrics::default();
        assert_eq!(metrics.snapshot().claimed_total, 0);
        assert_eq!(metrics.snapshot().dead_lettered_total, 0);
    }

    #[test]
    fn retry_delay_is_exponential_and_bounded() {
        assert_eq!(
            retry_delay(Duration::from_secs(5), 1),
            Duration::from_secs(5)
        );
        assert_eq!(
            retry_delay(Duration::from_secs(5), 3),
            Duration::from_secs(20)
        );
        assert_eq!(
            retry_delay(Duration::from_secs(5), 10),
            Duration::from_secs(300)
        );
        assert_eq!(
            retry_delay(Duration::from_secs(5), 100),
            Duration::from_secs(300)
        );
    }
}
