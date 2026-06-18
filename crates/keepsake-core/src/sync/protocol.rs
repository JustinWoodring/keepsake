//! Wire protocol types for the sync server.

use serde::{Deserialize, Serialize};

use super::VectorClock;

/// A request body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Request {
    /// Register a new user.  Body is the envelope public key.
    Register {
        /// The chosen username.
        username: String,
        /// 32-byte Ed25519 public key.
        envelope_pk: [u8; 32],
    },
    /// Sign in.  Body is the signature over a server challenge.
    Login {
        /// Username.
        username: String,
        /// Challenge bytes.
        challenge: Vec<u8>,
        /// Signature over `challenge`.
        signature: Vec<u8>,
    },
    /// Push a batch of changes.
    Push {
        /// Vector clock after applying these changes locally.
        new_clock: VectorClock,
        /// The changes themselves.
        changes: Vec<super::Change>,
    },
    /// Pull all changes since a vector clock (long-poll).
    Pull {
        /// The client's current clock.
        since: VectorClock,
    },
    /// Upload an opaque encrypted blob.
    PutBlob {
        /// SHA-256 of the plaintext (content addressing).
        content_sha256: [u8; 32],
        /// Ciphertext.
        ciphertext: Vec<u8>,
    },
    /// Fetch an opaque encrypted blob.
    GetBlob {
        /// SHA-256 of the plaintext.
        content_sha256: [u8; 32],
    },
}

/// A response body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    /// Generic OK with optional message.
    Ok {
        /// Optional message.
        message: Option<String>,
    },
    /// Login succeeded with a bearer.
    LoginOk {
        /// 32-byte bearer secret.
        bearer: [u8; 32],
        /// Expiry timestamp (Unix seconds).
        expires_at: i64,
    },
    /// Pull returned changes.
    PullResp {
        /// New changes the client should apply.
        changes: Vec<super::Change>,
        /// The server's current clock.
        server_clock: VectorClock,
    },
    /// GetBlob returned ciphertext.
    BlobResp {
        /// Ciphertext bytes.
        ciphertext: Vec<u8>,
    },
    /// Error.
    Error {
        /// Machine-readable error code.
        code: String,
        /// Human-readable message.
        message: String,
    },
}
