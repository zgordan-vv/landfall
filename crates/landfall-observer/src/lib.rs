//! Solana JSON-RPC observation and scheduling adapter.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use std::{cmp::Ordering, collections::BinaryHeap};

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

/// Solana RPC's maximum signature-status request size for one call.
pub const MAX_SIGNATURE_STATUS_BATCH: usize = 256;

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
}
