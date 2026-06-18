//! Transport abstraction and HTTP implementation.

pub mod envelope_auth;
pub mod http;

use async_trait::async_trait;

use crate::error::Result;
use crate::sync::protocol::{Request, Response};

pub use http::HttpTransport;

/// A pluggable transport for sync requests.  Implementations
/// send the [`Request`] to the server and return the [`Response`].
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a request and await the response.
    async fn round_trip(&self, req: &Request) -> Result<Response>;
}
