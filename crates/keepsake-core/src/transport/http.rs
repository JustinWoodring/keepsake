//! HTTP transport for sync.  In v1 this is a thin reqwest client
//! that POSTs JSON to `/v1/sync`.

use std::time::Duration;

use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::sync::protocol::{Request, Response};

use super::Transport;

/// HTTP transport.
pub struct HttpTransport {
    base: String,
    client: reqwest::Client,
    bearer: Option<[u8; 32]>,
}

impl HttpTransport {
    /// Build a transport pointing at `base_url` (e.g. `https://sync.example.com`).
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| Error::Transport(format!("reqwest build: {e}")))?;
        Ok(Self {
            base: base_url.into(),
            client,
            bearer: None,
        })
    }

    /// Install a bearer token for all subsequent requests.
    pub fn set_bearer(&mut self, bearer: [u8; 32]) {
        self.bearer = Some(bearer);
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn round_trip(&self, req: &Request) -> Result<Response> {
        let url = format!("{}/v1/sync", self.base.trim_end_matches('/'));
        let mut builder = self.client.post(&url).json(req);
        if let Some(b) = self.bearer {
            builder = builder.bearer_auth(hex::encode_upper(b));
        }
        let resp = builder
            .send()
            .await
            .map_err(|e| Error::Transport(format!("send: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::Transport(format!(
                "server returned {}",
                resp.status()
            )));
        }
        let body: Response = resp
            .json()
            .await
            .map_err(|e| Error::Transport(format!("decode: {e}")))?;
        Ok(body)
    }
}
