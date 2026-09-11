use std::time::Duration;

use async_trait::async_trait;

use crate::RpcTransport;

/// A route-owned pooled HTTP client. `reqwest::Client` reuses connections.
pub struct ReqwestRouteClient {
    route_id: String,
    endpoint: String,
    client: reqwest::Client,
}

impl ReqwestRouteClient {
    /// Builds a Rustls-only client for one configured RPC route.
    pub fn new(
        route_id: impl Into<String>,
        endpoint: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, reqwest::Error> {
        let route_id = route_id.into();
        let endpoint = endpoint.into();
        let client = reqwest::Client::builder()
            .use_rustls_tls()
            .pool_idle_timeout(Duration::from_secs(90))
            .timeout(timeout)
            .build()?;
        Ok(Self {
            route_id,
            endpoint,
            client,
        })
    }

    pub fn route_id(&self) -> &str {
        &self.route_id
    }
}

#[async_trait]
impl RpcTransport for ReqwestRouteClient {
    async fn post(&self, endpoint: &str, body: Vec<u8>) -> Result<Vec<u8>, String> {
        if endpoint != self.endpoint {
            return Err("rpc route endpoint mismatch".to_owned());
        }
        let response = self
            .client
            .post(&self.endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .map_err(|error| format!("http transport failed: {error}"))?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| format!("http body read failed: {error}"))?;
        if !status.is_success() {
            return Err(format!("http status {status}"));
        }
        Ok(bytes.to_vec())
    }
}
