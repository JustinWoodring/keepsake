//! Envelope-signed HTTP requests.

use crate::error::Result;
use crate::identity::EnvelopeKey;

/// A signed request envelope.  The `signature` is over the
/// canonical serialization of the body.
pub struct SignedRequest<B> {
    /// Envelope public key (hex in headers).
    pub envelope_pk: String,
    /// Signature over the body.
    pub signature: Vec<u8>,
    /// The body.
    pub body: B,
}

impl<B: serde::Serialize> SignedRequest<B> {
    /// Sign a body.
    pub fn sign(envelope: &EnvelopeKey, body: B) -> Result<Self> {
        let bytes = serde_json::to_vec(&body)
            .map_err(|e| crate::Error::Transport(format!("serialize: {e}")))?;
        let sig = envelope.signing_key().sign(&bytes);
        Ok(Self {
            envelope_pk: hex::encode_upper(envelope.public_key().to_bytes()),
            signature: sig.to_bytes().to_vec(),
            body,
        })
    }
}

/// Convenience: hex-encode a public key for the `X-Envelope-Pk` header.
pub fn pk_header(envelope: &EnvelopeKey) -> String {
    hex::encode_upper(envelope.public_key().to_bytes())
}
