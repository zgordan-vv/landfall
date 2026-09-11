//! Solana JSON-RPC observation and scheduling adapter.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

mod http;
pub use http::ReqwestRouteClient;

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
}
