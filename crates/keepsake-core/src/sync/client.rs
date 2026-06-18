//! High-level sync client.  Walks the local vault, encrypts
//! each record with the master key, and pushes to the server.
//! On pull, decrypts incoming payloads and applies them via
//! the CRDT layer.
//!
//! ## Usage
//!
//! ```ignore
//! use keepsake_core::sync::client::SyncClient;
//!
//! let client = SyncClient::new("https://sync.example.com");
//! // Register (idempotent).
//! client.register(&session).await?;
//! // Push local records.
//! let n = client.push(&session).await?;
//! // Pull remote changes (takes &mut because it mutates the
//! // local vault).
//! let n = client.pull(&mut session).await?;
//! # Ok::<(), keepsake_core::Error>(())
//! ```
//!
//! ## Payload format
//!
//! For each `Change`, the `payload` is:
//!
//! ```text
//! [nonce: 24 bytes][AEAD-ciphertext: variable]
//! ```
//!
//! The AEAD plaintext is `serde_json::to_vec(&record)`; the
//! key is the user's master key; the AAD is
//! `keepsake/sync/payload/v1`.  The server stores the bytes;
//! receivers decrypt and run the change through the CRDT
//! layer (see [`crate::sync::update::apply_change`]).

use std::sync::Arc;

use chrono::Utc;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::crypto::aead::{self, AeadKey, Nonce};
use crate::error::{Error, Result};
use crate::identity::EnvelopeKey;
use crate::records::{Record, RecordHeader, ALL_TYPES};
use crate::session::Session;
use crate::sync::protocol::{Request, Response as ProtoResponse};
use crate::sync::{Change, VectorClock};

const PAYLOAD_AAD: &[u8] = b"keepsake/sync/payload/v1";

/// HTTP sync client.  Cheap to construct; holds an HTTP client
/// and a cached bearer.  Holds *no* session — the session is
/// borrowed for the duration of each method call.  This lets
/// the caller keep the session behind a Mutex without
/// complicated ownership transfers.
pub struct SyncClient {
    server_url: String,
    bearer: Mutex<Option<Bearer>>,
    http: reqwest::Client,
}

#[derive(Clone)]
struct Bearer {
    bytes: [u8; 32],
    expires_at: i64,
}

impl SyncClient {
    /// Construct a client pointing at `server_url`.
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            bearer: Mutex::new(None),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client build"),
        }
    }

    /// Register the current user with the server.  Idempotent
    /// (returns Ok if already registered, i.e. 409).
    pub async fn register(&self, session: &Session) -> Result<()> {
        let envelope = EnvelopeKey::from_master_key(&session.master)?;
        let envelope_pk = envelope.public_key().to_bytes();
        let body = serde_json::to_vec(&Request::Register {
            username: session.username.clone(),
            envelope_pk,
        })?;
        let url = format!("{}/v1/auth/register", self.url_base());
        let resp = self.http
            .post(&url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("register: {e}")))?;
        if !resp.status().is_success() && resp.status().as_u16() != 409 {
            return Err(Error::Transport(format!(
                "register failed: {}",
                resp.status()
            )));
        }
        Ok(())
    }

    /// Push every record in the local vault to the server.
    /// Returns the number of records pushed.
    pub async fn push(&self, session: &Session) -> Result<usize> {
        // Build the change list.  We only need `&Session` here
        // (no mutations), so this is a simple borrow.
        let mut changes: Vec<Change> = Vec::new();
        let mut lamport: u64 = 0;
        for t in ALL_TYPES {
            for h in session.vault.list_records(t)? {
                let (_h2, rec) = session.vault.get_record(h.id)?;
                let payload = encrypt_record(&session.master, &rec)?;
                lamport += 1;
                let ch = Change {
                    id: Uuid::new_v4(),
                    lamport,
                    ts: Utc::now(),
                    author: session.username.clone(),
                    record_id: Some(h.id),
                    payload,
                };
                changes.push(ch);
            }
        }

        if changes.is_empty() {
            return Ok(0);
        }
        let n = changes.len();

        let body = serde_json::to_vec(&Request::Push {
            new_clock: VectorClock {
                counters: [(session.username.clone(), lamport)]
                    .into_iter()
                    .collect(),
            },
            changes,
        })?;
        self.send_signed(session, "/v1/sync/push", &body).await?;
        Ok(n)
    }

    /// Pull all changes from the server and apply them to the
    /// local vault.  Returns the number of changes applied.
    pub async fn pull(&self, session: &mut Session) -> Result<usize> {
        let body = serde_json::to_vec(&Request::Pull {
            since: VectorClock::default(),
        })?;
        let resp_bytes = self.send_signed(session, "/v1/sync/pull", &body).await?;
        let resp: ProtoResponse = serde_json::from_slice(&resp_bytes)?;
        let changes = match resp {
            ProtoResponse::PullResp { changes, .. } => changes,
            other => return Err(Error::Sync(format!("unexpected pull response: {other:?}"))),
        };

        let mut applied = 0;
        for ch in changes {
            apply_remote_change(session, &ch)?;
            applied += 1;
        }
        Ok(applied)
    }

    fn url_base(&self) -> String {
        self.server_url.trim_end_matches('/').to_string()
    }

    /// Send a signed request.  Refreshes the bearer on 401.
    async fn send_signed(
        &self,
        session: &Session,
        path: &str,
        body: &[u8],
    ) -> Result<Vec<u8>> {
        let envelope = EnvelopeKey::from_master_key(&session.master)?;
        for attempt in 0..2 {
            let bearer = self.ensure_bearer(session, &envelope).await?;
            let url = format!("{}{}", self.url_base(), path);
            let mut builder = self.http.post(&url);
            if let Some(b) = bearer {
                builder = builder.header(
                    "authorization",
                    format!("Bearer {}", hex::encode_upper(b)),
                );
            }
            let envelope_pk = envelope.public_key().to_bytes();
            let sig = envelope.signing_key().sign(body);
            let req = builder
                .header("content-type", "application/json")
                .header("x-envelope-pk", hex::encode_upper(envelope_pk))
                .header("x-signature", hex::encode_upper(sig.to_bytes()))
                .body(body.to_vec())
                .send()
                .await
                .map_err(|e| Error::Transport(format!("http: {e}")))?;
            let status = req.status();
            let bytes = req.bytes().await
                .map_err(|e| Error::Transport(format!("read body: {e}")))?
                .to_vec();

            if status.as_u16() == 401 && attempt == 0 {
                *self.bearer.lock() = None;
                continue;
            }
            if !status.is_success() {
                return Err(Error::Transport(format!(
                    "server returned {status}: {}",
                    String::from_utf8_lossy(&bytes)
                )));
            }
            return Ok(bytes);
        }
        Err(Error::Transport("auth retry exhausted".into()))
    }

    /// Ensure we have a valid bearer; obtain one if not.
    async fn ensure_bearer(
        &self,
        session: &Session,
        envelope: &EnvelopeKey,
    ) -> Result<Option<[u8; 32]>> {
        let cached = self.bearer.lock().clone();
        if let Some(b) = cached {
            if b.expires_at > Utc::now().timestamp() + 5 {
                return Ok(Some(b.bytes));
            }
        }

        // Step 1: request a challenge.
        let url = format!(
            "{}/v1/auth/challenge?username={}",
            self.url_base(),
            session.username
        );
        let resp = self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("challenge: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::Transport(format!(
                "challenge request failed: {}",
                resp.status()
            )));
        }
        let bytes = resp.bytes().await
            .map_err(|e| Error::Transport(format!("read body: {e}")))?
            .to_vec();
        let resp: ProtoResponse = serde_json::from_slice(&bytes)?;
        let challenge: [u8; 32] = match resp {
            ProtoResponse::BlobResp { ciphertext } => {
                if ciphertext.len() != 32 {
                    return Err(Error::Transport("bad challenge length".into()));
                }
                ciphertext.try_into().unwrap()
            }
            other => {
                return Err(Error::Sync(format!("unexpected challenge response: {other:?}")))
            }
        };

        // Step 2: sign the challenge and POST login.
        let sig = envelope.signing_key().sign(&challenge);
        let body = serde_json::to_vec(&Request::Login {
            username: session.username.clone(),
            challenge: challenge.to_vec(),
            signature: sig.to_bytes().to_vec(),
        })?;
        let url = format!("{}/v1/auth/login", self.url_base());
        let resp = self.http
            .post(&url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("login: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Transport(format!(
                "login failed: {status}: {body}"
            )));
        }
        let bytes = resp.bytes().await
            .map_err(|e| Error::Transport(format!("read body: {e}")))?
            .to_vec();
        let resp: ProtoResponse = serde_json::from_slice(&bytes)?;
        let (bearer, expires_at) = match resp {
            ProtoResponse::LoginOk { bearer, expires_at } => (bearer, expires_at),
            other => {
                return Err(Error::Sync(format!("unexpected login response: {other:?}")))
            }
        };
        *self.bearer.lock() = Some(Bearer { bytes: bearer, expires_at });
        Ok(Some(bearer))
    }
}

/// Encrypt a record for transport.  Returns
/// `nonce || ciphertext`.  Keyed by the master key.
fn encrypt_record(
    master: &crate::identity::MasterKey,
    rec: &Record,
) -> Result<Vec<u8>> {
    let key = AeadKey::from_bytes({
        let mut b = [0u8; 32];
        b.copy_from_slice(master.as_bytes());
        b
    });
    let plaintext = serde_json::to_vec(rec)?;
    let nonce = Nonce::random();
    let ct = aead::encrypt(&key, &nonce, &plaintext, PAYLOAD_AAD)?;
    let mut out = Vec::with_capacity(24 + ct.len());
    out.extend_from_slice(nonce.as_bytes());
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt a record payload from transport.
fn decrypt_record(
    master: &crate::identity::MasterKey,
    payload: &[u8],
) -> Result<Record> {
    if payload.len() < 24 {
        return Err(Error::Sync("payload too short".into()));
    }
    let (nonce_bytes, ct) = payload.split_at(24);
    let nonce_arr: [u8; 24] = nonce_bytes.try_into().unwrap();
    let nonce = Nonce::from_bytes(nonce_arr);
    let key = AeadKey::from_bytes({
        let mut b = [0u8; 32];
        b.copy_from_slice(master.as_bytes());
        b
    });
    let pt = aead::decrypt(&key, &nonce, ct, PAYLOAD_AAD)?;
    let rec: Record = serde_json::from_slice(&pt)?;
    Ok(rec)
}

/// Apply a remote change to the local vault.  Decrypts the
/// payload, then runs through the CRDT layer.
fn apply_remote_change(session: &mut Session, ch: &Change) -> Result<()> {
    let record_id = ch.record_id.ok_or_else(|| {
        Error::Sync("change has no record_id; only record changes are supported in v1".into())
    })?;
    let rec = decrypt_record(&session.master, &ch.payload)?;

    let header = RecordHeader {
        r#type: rec.type_tag().to_string(),
        schema_version: rec.schema_version(),
        id: record_id,
        created_by: ch.author.clone(),
        updated_by: ch.author.clone(),
        created_at: ch.ts,
        updated_at: ch.ts,
    };

    let encoded = crate::sync::update::EncodedRecord {
        header,
        record: rec,
    };
    let encoded_bytes = serde_json::to_vec(&encoded)?;
    let outcome = crate::sync::update::apply_change(&mut session.vault, &Change {
        id: ch.id,
        lamport: ch.lamport,
        ts: ch.ts,
        author: ch.author.clone(),
        record_id: ch.record_id,
        payload: encoded_bytes,
    })?;
    let _ = outcome;
    Ok(())
}

// Suppress unused-import warning.
#[allow(dead_code)]
fn _arc_unused(_: Arc<()>) {}
