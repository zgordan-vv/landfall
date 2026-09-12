//! Solana JSON-RPC observation and scheduling adapter.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{cmp::Ordering, collections::BinaryHeap};
use tokio_util::sync::CancellationToken;

mod http;
pub use http::ReqwestRouteClient;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ObservationSchedule {
    pub job_id: String,
    pub trace_id: String,
    pub due_at: Instant,
    pub priority: u8,
}

impl Ord for ObservationSchedule {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .due_at
            .cmp(&self.due_at)
            .then_with(|| self.priority.cmp(&other.priority))
            .then_with(|| other.job_id.cmp(&self.job_id))
    }
}
impl PartialOrd for ObservationSchedule {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// In-memory due-time queue; durable jobs are re-enqueued after process restart.
pub struct ObservationQueue {
    entries: BinaryHeap<ObservationSchedule>,
}

/// Cooperative cancellation handle shared by observer workers.
#[derive(Clone)]
pub struct ObserverCancellation {
    token: CancellationToken,
}

impl ObserverCancellation {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
        }
    }
    pub fn cancel(&self) {
        self.token.cancel();
    }
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
    pub async fn cancelled(&self) {
        self.token.cancelled().await;
    }
}

impl Default for ObserverCancellation {
    fn default() -> Self {
        Self::new()
    }
}

/// Rehydrates ready durable schedules into a fresh process-local queue.
pub fn rehydrate_queue(
    queue: &mut ObservationQueue,
    schedules: impl IntoIterator<Item = ObservationSchedule>,
) {
    for schedule in schedules {
        queue.push(schedule);
    }
}

/// Solana RPC's maximum signature-status request size for one call.
pub const MAX_SIGNATURE_STATUS_BATCH: usize = 256;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RouteHealth {
    pub requests: u64,
    pub successes: u64,
    pub failures: u64,
    pub rate_limited: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataQualityGap {
    NoStatusEvidence,
    RpcUnavailable,
    MalformedProviderResponse,
    UnsupportedTransactionVersion,
}

/// Aggregates health independently for each configured observer route.
#[derive(Debug, Default)]
pub struct RouteHealthRegistry {
    routes: std::collections::HashMap<String, RouteHealth>,
}

impl RouteHealthRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn record(&mut self, route_id: &str, result: Result<(), &str>) {
        let health = self.routes.entry(route_id.to_owned()).or_default();
        health.requests += 1;
        match result {
            Ok(()) => health.successes += 1,
            Err("rate_limited") => {
                health.failures += 1;
                health.rate_limited += 1;
            }
            Err(_) => health.failures += 1,
        }
    }
    pub fn get(&self, route_id: &str) -> RouteHealth {
        self.routes.get(route_id).copied().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpirationDecision {
    NotExpired,
    Expired,
    IndeterminateDurableNonce,
    UnsupportedValidity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidityCase {
    RecentBlockhash,
    DurableNonce,
    Unsupported,
}

pub fn classify_validity(transaction_version: &str, uses_durable_nonce: bool) -> ValidityCase {
    if uses_durable_nonce {
        ValidityCase::DurableNonce
    } else if matches!(transaction_version, "legacy" | "v0") {
        ValidityCase::RecentBlockhash
    } else {
        ValidityCase::Unsupported
    }
}

pub fn evaluate_validity(
    case: ValidityCase,
    current_block_height: Option<u64>,
    last_valid_block_height: Option<u64>,
) -> ExpirationDecision {
    match case {
        ValidityCase::DurableNonce => ExpirationDecision::IndeterminateDurableNonce,
        ValidityCase::Unsupported => ExpirationDecision::UnsupportedValidity,
        ValidityCase::RecentBlockhash => match (current_block_height, last_valid_block_height) {
            (Some(current), Some(last_valid)) => evaluate_expiration(current, last_valid, false),
            _ => ExpirationDecision::NotExpired,
        },
    }
}

/// Evaluates recent-blockhash validity without applying it to durable-nonce transactions.
pub fn evaluate_expiration(
    current_block_height: u64,
    last_valid_block_height: u64,
    durable_nonce: bool,
) -> ExpirationDecision {
    if durable_nonce {
        ExpirationDecision::IndeterminateDurableNonce
    } else if current_block_height > last_valid_block_height {
        ExpirationDecision::Expired
    } else {
        ExpirationDecision::NotExpired
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ObservationEvidence {
    FirstNull {
        observed_at: String,
    },
    StatusChanged {
        from: Option<String>,
        to: String,
        observed_at: String,
    },
    Checkpoint {
        observed_at: String,
        block_height: Option<u64>,
    },
    RpcErrorTransition {
        previous: Option<String>,
        current: String,
        observed_at: String,
    },
    Terminal {
        outcome: String,
        observed_at: String,
    },
}

/// Append-only evidence accumulator; callers persist its records durably.
#[derive(Debug, Default)]
pub struct ObservationEvidenceLog {
    records: Vec<ObservationEvidence>,
    last_status: Option<String>,
    saw_null: bool,
    last_rpc_error: Option<String>,
}

impl ObservationEvidenceLog {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn records(&self) -> &[ObservationEvidence] {
        &self.records
    }
    pub fn record_status(&mut self, status: Option<&str>, observed_at: impl Into<String>) {
        let observed_at = observed_at.into();
        if status.is_none() && !self.saw_null {
            self.saw_null = true;
            self.records.push(ObservationEvidence::FirstNull {
                observed_at: observed_at.clone(),
            });
        }
        if let Some(status) = status {
            if self.last_status.as_deref() != Some(status) {
                let from = self.last_status.replace(status.to_owned());
                self.records.push(ObservationEvidence::StatusChanged {
                    from,
                    to: status.to_owned(),
                    observed_at,
                });
            }
        }
    }
    pub fn checkpoint(&mut self, observed_at: impl Into<String>, block_height: Option<u64>) {
        self.records.push(ObservationEvidence::Checkpoint {
            observed_at: observed_at.into(),
            block_height,
        });
    }
    pub fn record_rpc_error(&mut self, error: Option<&str>, observed_at: impl Into<String>) {
        if self.last_rpc_error.as_deref() != error {
            let previous = self.last_rpc_error.clone();
            self.last_rpc_error = error.map(str::to_owned);
            if let Some(current) = error {
                self.records.push(ObservationEvidence::RpcErrorTransition {
                    previous,
                    current: current.to_owned(),
                    observed_at: observed_at.into(),
                });
            }
        }
    }
    pub fn terminal(&mut self, outcome: impl Into<String>, observed_at: impl Into<String>) {
        self.records.push(ObservationEvidence::Terminal {
            outcome: outcome.into(),
            observed_at: observed_at.into(),
        });
    }
}

/// Bounded exponential polling policy shared by observation jobs.
#[derive(Debug, Clone, Copy)]
pub struct AdaptivePollPolicy {
    pub initial: Duration,
    pub maximum: Duration,
}

impl AdaptivePollPolicy {
    pub fn new(initial: Duration, maximum: Duration) -> Result<Self, &'static str> {
        if initial.is_zero() || maximum < initial {
            return Err("polling maximum must be at least the non-zero initial interval");
        }
        Ok(Self { initial, maximum })
    }

    pub fn delay(&self, attempt: u32, rate_limited: bool) -> Duration {
        let exponent = attempt.min(16);
        let multiplier = 1u32.checked_shl(exponent).unwrap_or(u32::MAX);
        let base = self.initial.saturating_mul(multiplier);
        let delayed = if rate_limited {
            base.saturating_mul(2)
        } else {
            base
        };
        delayed.min(self.maximum)
    }
}

/// Per-route minimum interval limiter; each route has an independent budget.
pub struct RouteRateLimiter {
    interval: Duration,
    next_allowed: Mutex<Instant>,
}

impl RouteRateLimiter {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            next_allowed: Mutex::new(Instant::now()),
        }
    }

    pub async fn wait(&self) {
        let delay = {
            let now = Instant::now();
            let mut next = self.next_allowed.lock().expect("rate limiter mutex");
            let delay = next.saturating_duration_since(now);
            *next = now.max(*next) + self.interval;
            delay
        };
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
}

#[derive(Debug, Deserialize)]
struct SignatureStatusesResponse {
    value: Vec<Option<serde_json::Value>>,
}

/// Fetches statuses in bounded chunks while preserving input ordering.
pub async fn get_signature_statuses<T: RpcTransport>(
    client: &JsonRpcClient<T>,
    signatures: &[String],
) -> Result<Vec<Option<serde_json::Value>>, RpcClientError> {
    if signatures.is_empty() {
        return Ok(Vec::new());
    }
    let mut statuses = Vec::with_capacity(signatures.len());
    for chunk in signatures.chunks(MAX_SIGNATURE_STATUS_BATCH) {
        let response: SignatureStatusesResponse = client
            .call(1, "getSignatureStatuses", serde_json::json!([chunk]))
            .await?;
        statuses.extend(response.value);
    }
    Ok(statuses)
}

/// Fetches optional execution details while explicitly supporting legacy and v0 transactions.
pub async fn get_transaction<T: RpcTransport>(
    client: &JsonRpcClient<T>,
    signature: &str,
) -> Result<Option<serde_json::Value>, RpcClientError> {
    if signature.trim().is_empty() {
        return Err(RpcClientError::MalformedResponse);
    }
    client
        .call(
            1,
            "getTransaction",
            serde_json::json!([signature, { "maxSupportedTransactionVersion": 0 }]),
        )
        .await
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedExecution {
    pub slot: String,
    pub block_time: Option<i64>,
    pub fee_lamports: Option<String>,
    pub compute_units_consumed: Option<String>,
    pub transaction_version: String,
    pub logs_present: bool,
    pub execution_error: bool,
}

/// Converts provider-specific transaction JSON into bounded neutral evidence.
pub fn normalize_execution(
    value: &serde_json::Value,
) -> Result<NormalizedExecution, RpcClientError> {
    let object = value.as_object().ok_or(RpcClientError::MalformedResponse)?;
    let slot = object
        .get("slot")
        .and_then(serde_json::Value::as_u64)
        .ok_or(RpcClientError::MalformedResponse)?
        .to_string();
    let meta = object.get("meta").and_then(serde_json::Value::as_object);
    let fee_lamports = meta
        .and_then(|m| m.get("fee"))
        .and_then(serde_json::Value::as_u64)
        .map(|v| v.to_string());
    let compute_units_consumed = meta
        .and_then(|m| m.get("computeUnitsConsumed"))
        .and_then(serde_json::Value::as_u64)
        .map(|v| v.to_string());
    let logs_present = meta
        .and_then(|m| m.get("logMessages"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|logs| !logs.is_empty());
    let execution_error = meta
        .and_then(|m| m.get("err"))
        .is_some_and(|error| !error.is_null());
    let transaction_version = match object.get("version") {
        None => "legacy",
        Some(value) if value.as_str() == Some("legacy") => "legacy",
        Some(value) if value.as_str() == Some("0") || value.as_u64() == Some(0) => "v0",
        Some(_) => "unsupported",
    }
    .to_owned();
    Ok(NormalizedExecution {
        slot,
        block_time: object.get("blockTime").and_then(serde_json::Value::as_i64),
        fee_lamports,
        compute_units_consumed,
        transaction_version,
        logs_present,
        execution_error,
    })
}

impl ObservationQueue {
    pub fn new() -> Self {
        Self {
            entries: BinaryHeap::new(),
        }
    }
    pub fn push(&mut self, schedule: ObservationSchedule) {
        self.entries.push(schedule);
    }
    pub fn pop_due(&mut self, now: Instant) -> Option<ObservationSchedule> {
        self.entries.peek().filter(|entry| entry.due_at <= now)?;
        self.entries.pop()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

impl Default for ObservationQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
struct CachedHeight {
    value: u64,
    fetched_at: Instant,
}

/// Shared short-lived block-height cache for observer jobs on one route.
pub struct BlockHeightCache {
    ttl: Duration,
    value: Mutex<Option<CachedHeight>>,
}

impl BlockHeightCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            value: Mutex::new(None),
        }
    }

    pub async fn get_or_refresh<T: RpcTransport>(
        &self,
        client: &JsonRpcClient<T>,
    ) -> Result<u64, RpcClientError> {
        if let Some(cached) = *self
            .value
            .lock()
            .map_err(|_| RpcClientError::MalformedResponse)?
        {
            if cached.fetched_at.elapsed() < self.ttl {
                return Ok(cached.value);
            }
        }
        let height: u64 = client.call(1, "getBlockHeight", Vec::<u8>::new()).await?;
        let mut slot = self
            .value
            .lock()
            .map_err(|_| RpcClientError::MalformedResponse)?;
        *slot = Some(CachedHeight {
            value: height,
            fetched_at: Instant::now(),
        });
        Ok(height)
    }
}

#[derive(Debug, Serialize)]
struct RpcRequest<'a, P> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: P,
}

#[derive(Debug, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcErrorBody>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcErrorBody {
    pub code: i64,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum RpcClientError {
    #[error("rpc transport failed: {0}")]
    Transport(String),
    #[error("rpc provider returned {code}: {message}")]
    Provider { code: i64, message: String },
    #[error("rpc response was malformed")]
    MalformedResponse,
}

/// Normalized subset of Solana `getSignatureStatuses` response.
#[derive(Debug, Clone, Deserialize)]
pub struct SignatureStatus {
    pub slot: u64,
    pub confirmations: Option<u64>,
    pub err: Option<serde_json::Value>,
    #[serde(rename = "confirmationStatus")]
    pub confirmation_status: Option<String>,
}

/// Deterministic interpretation of one signature status response.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NormalizedSignatureObservation {
    pub source_result: &'static str,
    pub commitment: Option<String>,
    pub slot: Option<u64>,
    pub execution_error: Option<serde_json::Value>,
}

/// Builds the canonical wire payload for a status observation event.
pub fn status_observed_event(
    project_id: &str,
    environment_id: &str,
    trace_id: &str,
    observer_source_id: &str,
    event_id: &str,
    occurred_at: &str,
    observation: &NormalizedSignatureObservation,
) -> serde_json::Value {
    let mut attributes = serde_json::json!({
        "observer_source_id": observer_source_id,
        "source_result": observation.source_result,
        "duration_ns": "0"
    });
    if let Some(commitment) = &observation.commitment {
        attributes["commitment"] = serde_json::Value::String(commitment.clone());
    }
    if let Some(slot) = observation.slot {
        attributes["slot"] = serde_json::Value::String(slot.to_string());
    }
    serde_json::json!({"schema_version":"1.0","event_type":"solana.status.observed","event_id":event_id,"occurred_at":occurred_at,"project_id":project_id,"environment_id":environment_id,"trace_id":trace_id,"source":{"kind":"observer","name":"landfall-observer","version":"0.1.0"},"privacy_mode":"standard","privacy_policy_version":"1.0","redaction_version":"1.0","attributes":attributes})
}

/// Maps a Solana RPC status to protocol-level observation fields.
#[must_use]
pub fn normalize_signature_status(
    status: Option<SignatureStatus>,
) -> NormalizedSignatureObservation {
    match status {
        None => NormalizedSignatureObservation {
            source_result: "not_found",
            commitment: None,
            slot: None,
            execution_error: None,
        },
        Some(status) => NormalizedSignatureObservation {
            source_result: "found",
            commitment: status.confirmation_status,
            slot: Some(status.slot),
            execution_error: status.err,
        },
    }
}

#[async_trait]
pub trait RpcTransport: Send + Sync {
    async fn post(&self, endpoint: &str, body: Vec<u8>) -> Result<Vec<u8>, String>;
}

pub struct JsonRpcClient<T> {
    endpoint: String,
    transport: T,
}

impl<T> JsonRpcClient<T> {
    pub fn new(endpoint: impl Into<String>, transport: T) -> Self {
        Self {
            endpoint: endpoint.into(),
            transport,
        }
    }
}

impl<T: RpcTransport> JsonRpcClient<T> {
    pub async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        id: u64,
        method: &str,
        params: P,
    ) -> Result<R, RpcClientError> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id,
            method,
            params,
        };
        let body = serde_json::to_vec(&request).map_err(|_| RpcClientError::MalformedResponse)?;
        let raw = self
            .transport
            .post(&self.endpoint, body)
            .await
            .map_err(RpcClientError::Transport)?;
        let response: RpcResponse<R> =
            serde_json::from_slice(&raw).map_err(|_| RpcClientError::MalformedResponse)?;
        match (response.result, response.error) {
            (Some(result), None) => Ok(result),
            (None, Some(error)) => Err(RpcClientError::Provider {
                code: error.code,
                message: error.message,
            }),
            _ => Err(RpcClientError::MalformedResponse),
        }
    }

    /// Reads one signature status from the configured RPC route.
    pub async fn get_signature_status(
        &self,
        signature: &str,
    ) -> Result<Option<SignatureStatus>, RpcClientError> {
        let result: SignatureStatuses = self
            .call(
                1,
                "getSignatureStatuses",
                (
                    vec![signature],
                    serde_json::json!({"searchTransactionHistory": true}),
                ),
            )
            .await?;
        Ok(result.value.into_iter().next().flatten())
    }
}

#[derive(Debug, Deserialize)]
struct SignatureStatuses {
    value: Vec<Option<SignatureStatus>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeTransport {
        response: Vec<u8>,
    }
    #[async_trait]
    impl RpcTransport for FakeTransport {
        async fn post(&self, endpoint: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
            assert_eq!(endpoint, "https://rpc.example");
            let request: serde_json::Value =
                serde_json::from_slice(&body).map_err(|error| error.to_string())?;
            assert_eq!(request["method"], "getBlockHeight");
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn provider_neutral_call_decodes_result() {
        let client = JsonRpcClient::new(
            "https://rpc.example",
            FakeTransport {
                response: br#"{"jsonrpc":"2.0","id":7,"result":123}"#.to_vec(),
            },
        );
        let result: u64 = client
            .call(7, "getBlockHeight", Vec::<u8>::new())
            .await
            .expect("result");
        assert_eq!(result, 123);
    }

    #[tokio::test]
    async fn provider_errors_are_normalized() {
        let client = JsonRpcClient::new(
            "https://rpc.example",
            FakeTransport {
                response:
                    br#"{"jsonrpc":"2.0","id":7,"error":{"code":-32005,"message":"rate limit"}}"#
                        .to_vec(),
            },
        );
        let error = client
            .call::<_, u64>(7, "getBlockHeight", Vec::<u8>::new())
            .await
            .expect_err("error");
        assert!(matches!(
            error,
            RpcClientError::Provider { code: -32005, .. }
        ));
    }

    #[tokio::test]
    async fn block_height_is_shared_until_ttl_expires() {
        let client = JsonRpcClient::new(
            "https://rpc.example",
            FakeTransport {
                response: br#"{"jsonrpc":"2.0","id":1,"result":321}"#.to_vec(),
            },
        );
        let cache = BlockHeightCache::new(Duration::from_secs(60));
        assert_eq!(cache.get_or_refresh(&client).await.expect("height"), 321);
        assert_eq!(
            cache.get_or_refresh(&client).await.expect("cached height"),
            321
        );
    }

    struct StatusTransport;
    #[async_trait]
    impl RpcTransport for StatusTransport {
        async fn post(&self, _: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
            let request: serde_json::Value =
                serde_json::from_slice(&body).map_err(|error| error.to_string())?;
            let count = request["params"][0].as_array().map_or(0, Vec::len);
            Ok(serde_json::json!({ "jsonrpc": "2.0", "id": 1, "result": { "value": (0..count).map(|_| serde_json::json!({"confirmations": 1})).collect::<Vec<_>>() } }).to_string().into_bytes())
        }
    }

    #[tokio::test]
    async fn signature_statuses_are_chunked_at_256() {
        let client = JsonRpcClient::new("https://rpc.example", StatusTransport);
        let signatures = (0..257)
            .map(|index| format!("sig-{index}"))
            .collect::<Vec<_>>();
        let statuses = get_signature_statuses(&client, &signatures)
            .await
            .expect("statuses");
        assert_eq!(statuses.len(), 257);
    }

    struct TransactionTransport;
    #[async_trait]
    impl RpcTransport for TransactionTransport {
        async fn post(&self, _: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
            let request: serde_json::Value =
                serde_json::from_slice(&body).map_err(|error| error.to_string())?;
            assert_eq!(request["method"], "getTransaction");
            assert_eq!(request["params"][1]["maxSupportedTransactionVersion"], 0);
            Ok(br#"{"jsonrpc":"2.0","id":1,"result":{"slot":42}}"#.to_vec())
        }
    }

    #[tokio::test]
    async fn transaction_details_request_enables_v0() {
        let client = JsonRpcClient::new("https://rpc.example", TransactionTransport);
        let result = get_transaction(&client, "signature")
            .await
            .expect("transaction")
            .expect("details");
        assert_eq!(result["slot"], 42);
    }

    #[test]
    fn execution_details_are_normalized_with_exact_integer_fields() {
        let value = serde_json::json!({ "slot": 42, "blockTime": 1_700_000_000_i64, "version": "0", "meta": { "fee": 5000, "computeUnitsConsumed": 900, "logMessages": ["ok"], "err": null } });
        let normalized = normalize_execution(&value).expect("execution");
        assert_eq!(normalized.slot, "42");
        assert_eq!(normalized.fee_lamports.as_deref(), Some("5000"));
        assert_eq!(normalized.compute_units_consumed.as_deref(), Some("900"));
        assert_eq!(normalized.transaction_version, "v0");
        assert!(!normalized.execution_error);
    }

    #[test]
    fn queue_returns_due_entries_by_time_then_priority() {
        let now = Instant::now();
        let mut queue = ObservationQueue::new();
        queue.push(ObservationSchedule {
            job_id: "late".into(),
            trace_id: "t2".into(),
            due_at: now + Duration::from_secs(5),
            priority: 9,
        });
        queue.push(ObservationSchedule {
            job_id: "low".into(),
            trace_id: "t1".into(),
            due_at: now,
            priority: 1,
        });
        queue.push(ObservationSchedule {
            job_id: "high".into(),
            trace_id: "t3".into(),
            due_at: now,
            priority: 9,
        });
        assert_eq!(queue.pop_due(now).expect("high").job_id, "high");
        assert_eq!(queue.pop_due(now).expect("low").job_id, "low");
        assert!(queue.pop_due(now).is_none());
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn polling_delay_grows_but_stays_bounded() {
        let policy = AdaptivePollPolicy::new(Duration::from_millis(100), Duration::from_secs(1))
            .expect("policy");
        assert_eq!(policy.delay(0, false), Duration::from_millis(100));
        assert_eq!(policy.delay(2, false), Duration::from_millis(400));
        assert_eq!(policy.delay(2, true), Duration::from_millis(800));
        assert_eq!(policy.delay(10, false), Duration::from_secs(1));
    }

    #[tokio::test]
    async fn route_limiter_allows_independent_routes() {
        let first = RouteRateLimiter::new(Duration::ZERO);
        let second = RouteRateLimiter::new(Duration::ZERO);
        first.wait().await;
        second.wait().await;
    }

    #[test]
    fn evidence_log_records_first_null_transitions_checkpoints_and_terminal() {
        let mut log = ObservationEvidenceLog::new();
        log.record_status(None, "t1");
        log.record_status(None, "t2");
        log.record_status(Some("processed"), "t3");
        log.record_status(Some("confirmed"), "t4");
        log.record_rpc_error(Some("rate_limited"), "t5");
        log.checkpoint("t6", Some(42));
        log.terminal("confirmed", "t7");
        assert_eq!(log.records().len(), 6);
        assert!(matches!(
            log.records()[0],
            ObservationEvidence::FirstNull { .. }
        ));
        assert!(matches!(
            log.records()[5],
            ObservationEvidence::Terminal { .. }
        ));
    }

    #[test]
    fn expiration_uses_strictly_greater_height_and_handles_nonce() {
        assert_eq!(
            evaluate_expiration(500, 500, false),
            ExpirationDecision::NotExpired
        );
        assert_eq!(
            evaluate_expiration(501, 500, false),
            ExpirationDecision::Expired
        );
        assert_eq!(
            evaluate_expiration(9_999, 1, true),
            ExpirationDecision::IndeterminateDurableNonce
        );
    }

    #[test]
    fn validity_cases_never_false_expire_nonce_or_unsupported_versions() {
        assert_eq!(
            classify_validity("legacy", false),
            ValidityCase::RecentBlockhash
        );
        assert_eq!(
            classify_validity("legacy", true),
            ValidityCase::DurableNonce
        );
        assert_eq!(classify_validity("v1", false), ValidityCase::Unsupported);
        assert_eq!(
            evaluate_validity(ValidityCase::DurableNonce, Some(900), Some(1)),
            ExpirationDecision::IndeterminateDurableNonce
        );
        assert_eq!(
            evaluate_validity(ValidityCase::Unsupported, Some(900), Some(1)),
            ExpirationDecision::UnsupportedValidity
        );
    }

    #[tokio::test]
    async fn cancellation_is_cooperative_and_queue_can_be_rehydrated() {
        let cancellation = ObserverCancellation::new();
        let waiter = cancellation.clone();
        let task = tokio::spawn(async move {
            waiter.cancelled().await;
        });
        cancellation.cancel();
        task.await.expect("cancellation waiter");
        assert!(cancellation.is_cancelled());
        let mut queue = ObservationQueue::new();
        rehydrate_queue(
            &mut queue,
            [ObservationSchedule {
                job_id: "recovered".into(),
                trace_id: "trace".into(),
                due_at: Instant::now(),
                priority: 1,
            }],
        );
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn route_health_isolated_and_rate_limits_are_visible() {
        let mut registry = RouteHealthRegistry::new();
        registry.record("primary", Ok(()));
        registry.record("primary", Err("rate_limited"));
        registry.record("backup", Err("timeout"));
        assert_eq!(
            registry.get("primary"),
            RouteHealth {
                requests: 2,
                successes: 1,
                failures: 1,
                rate_limited: 1
            }
        );
        assert_eq!(registry.get("backup").failures, 1);
        assert_eq!(registry.get("missing"), RouteHealth::default());
    }
}
