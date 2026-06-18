//! High-level sync client.  Walks the local vault, encrypts
//! each record with the supplied key, and pushes to the
//! server.  On pull, decrypts incoming payloads and applies
//! them via the CRDT layer.
//!
//! The client talks to the new vault-scoped server
//! (`/v1/vaults/:id/sync/{push,pull}`).  It does **not**
//! authenticate to the server — the server is a dumb
//! encrypted blob store, and knowing the URL + vault id is
//! the only access control.  The encryption key held by
//! the client is the only thing protecting the data.
//!
//! ## Usage
//!
//! ```ignore
//! use keepsake_core::sync::client::SyncClient;
//!
//! let client = SyncClient::new("https://sync.example.com", "personal");
//! // For the personal vault, derive the key from the master.
//! let key = derive_vault_key(&session.master)?;
//! // Push local records.
//! let n = client.push(&key, &session).await?;
//! // Pull remote changes (takes &mut because it mutates
//! // the local vault).
//! let n = client.pull(&mut key, &mut session).await?;
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
//! key is the supplied `AeadKey`; the AAD is
//! `keepsake/sync/payload/v1`.  The server stores the bytes;
//! receivers decrypt and run the change through the CRDT
//! layer (see [`crate::sync::update::apply_change`]).

use chrono::Utc;
use uuid::Uuid;

use crate::crypto::aead::{self, AeadKey, Nonce};
use crate::error::{Error, Result};
use crate::records::{Record, RecordHeader, ALL_TYPES};
use crate::session::Session;
use crate::sync::protocol::{Request as ProtoRequest, Response as ProtoResponse};
use crate::sync::{Change, VectorClock};

const PAYLOAD_AAD: &[u8] = b"keepsake/sync/payload/v1";

/// HTTP sync client pointed at a single server + vault id.
pub struct SyncClient {
    server_url: String,
    vault_id: String,
    http: reqwest::Client,
}

impl SyncClient {
    /// Construct a client for a given server + vault id.
    pub fn new(server_url: impl Into<String>, vault_id: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            vault_id: vault_id.into(),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("reqwest client build"),
        }
    }

    fn url_base(&self) -> String {
        self.server_url.trim_end_matches('/').to_string()
    }

    /// Push every record in the local vault to the server.
    /// `key` is the vault key (32 bytes) used to encrypt
    /// each record.  For a personal vault, derive this from
    /// the master key; for a shared vault, derive it from
    /// the shared passphrase.
    pub async fn push(&self, key: &AeadKey, session: &Session) -> Result<usize> {
        let mut changes: Vec<Change> = Vec::new();
        let mut lamport: u64 = 0;
        for t in ALL_TYPES {
            for h in session.vault.list_records(t)? {
                let (_h2, rec) = session.vault.get_record(h.id)?;
                let payload = encrypt_record(key, &rec)?;
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
        let body = serde_json::to_vec(&ProtoRequest::Push {
            new_clock: VectorClock {
                counters: [(session.username.clone(), lamport)]
                    .into_iter()
                    .collect(),
            },
            changes,
        })?;
        let url = format!("{}/v1/vaults/{}/sync/push", self.url_base(), self.vault_id);
        let resp = self.http
            .post(&url)
            .header("content-type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Transport(format!("push: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let bytes = resp.bytes().await
                .map_err(|e| Error::Transport(format!("read body: {e}")))?
                .to_vec();
            return Err(Error::Transport(format!(
                "push failed: {status}: {}",
                String::from_utf8_lossy(&bytes)
            )));
        }
        Ok(n)
    }

    /// Pull the current state of the vault from the server
    /// and apply the changes to the local vault.  Returns
    /// the number of changes applied.
    pub async fn pull(&self, key: &AeadKey, session: &mut Session) -> Result<usize> {
        let url = format!("{}/v1/vaults/{}/sync/pull", self.url_base(), self.vault_id);
        let resp = self.http
            .post(&url)
            .header("content-type", "application/json")
            // Empty body = current-state pull.
            .body(Vec::new())
            .send()
            .await
            .map_err(|e| Error::Transport(format!("pull: {e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let bytes = resp.bytes().await
                .map_err(|e| Error::Transport(format!("read body: {e}")))?
                .to_vec();
            return Err(Error::Transport(format!(
                "pull failed: {status}: {}",
                String::from_utf8_lossy(&bytes)
            )));
        }
        let bytes = resp.bytes().await
            .map_err(|e| Error::Transport(format!("read body: {e}")))?
            .to_vec();
        let resp: ProtoResponse = serde_json::from_slice(&bytes)?;
        let changes = match resp {
            ProtoResponse::PullResp { changes, .. } => changes,
            other => return Err(Error::Sync(format!("unexpected pull response: {other:?}"))),
        };

        let mut applied = 0;
        for ch in changes {
            apply_remote_change(key, session, &ch)?;
            applied += 1;
        }
        Ok(applied)
    }
}

/// Derive a vault key from a shared passphrase + vault id
/// using Argon2id.  Anyone with the same passphrase + vault
/// id will derive the same key.
pub fn derive_shared_key(passphrase: &[u8], vault_id: &str) -> Result<AeadKey> {
    use crate::crypto::{derive_master_key, hkdf::derive_subkey32, KdfParams};

    // Use a 16-byte deterministic salt derived from the
    // vault id.  Anyone re-deriving the key from the same
    // passphrase + vault id will land on the same salt.
    let mut salt = [0u8; 16];
    let mut h = blake3::Hasher::new();
    h.update(b"keepsake/shared-vault/v1\n");
    h.update(vault_id.as_bytes());
    let digest = h.finalize();
    salt.copy_from_slice(&digest.as_bytes()[..16]);

    let params = KdfParams {
        m_kib: 8 * 1024, // 8 MiB
        t: 3,
        p: 1,
    };
    let master = derive_master_key(passphrase, &salt, params)?;

    // Derive a sub-vault key from the master.  This gives us
    // a 32-byte derived key without exposing the master.
    let derived = derive_subkey32(master.as_bytes(), &salt, b"keepsake/shared-vault/key/v1")?;
    Ok(AeadKey::from_bytes(derived))
}

/// Derive the per-vault key from a personal master key.
/// Equivalent to the local vault key used for record
/// encryption on disk.
pub fn derive_personal_vault_key(
    master: &crate::identity::MasterKey,
) -> Result<AeadKey> {
    // Use HKDF with a fixed salt to derive a 32-byte key
    // from the master.  The local vault uses a different
    // path (it generates a random vault key at init and
    // seals it under the master); for the personal sync
    // path we want a deterministic derivation so the same
    // master always produces the same sync key.
    use crate::crypto::hkdf::derive_subkey32;
    let derived = derive_subkey32(
        master.as_bytes(),
        b"keepsake/personal-sync/v1",
        b"keepsake/personal-sync/key/v1",
    )?;
    Ok(AeadKey::from_bytes(derived))
}

/// Encrypt a record for transport.  Returns
/// `nonce || ciphertext`.  Keyed by the supplied `key`.
fn encrypt_record(key: &AeadKey, rec: &Record) -> Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(rec)?;
    let nonce = Nonce::random();
    let ct = aead::encrypt(key, &nonce, &plaintext, PAYLOAD_AAD)?;
    let mut out = Vec::with_capacity(24 + ct.len());
    out.extend_from_slice(nonce.as_bytes());
    out.extend_from_slice(&ct);
    Ok(out)
}

/// Decrypt a record payload from transport.
fn decrypt_record(key: &AeadKey, payload: &[u8]) -> Result<Record> {
    if payload.len() < 24 {
        return Err(Error::Sync("payload too short".into()));
    }
    let (nonce_bytes, ct) = payload.split_at(24);
    let nonce_arr: [u8; 24] = nonce_bytes.try_into().unwrap();
    let nonce = Nonce::from_bytes(nonce_arr);
    let pt = aead::decrypt(key, &nonce, ct, PAYLOAD_AAD)?;
    let rec: Record = serde_json::from_slice(&pt)?;
    Ok(rec)
}

/// Apply a remote change to the local vault.  Decrypts the
/// payload, then runs through the CRDT layer.
fn apply_remote_change(key: &AeadKey, session: &mut Session, ch: &Change) -> Result<()> {
    let record_id = ch.record_id.ok_or_else(|| {
        Error::Sync("change has no record_id; only record changes are supported in v1".into())
    })?;
    let rec = decrypt_record(key, &ch.payload)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_key_derivation_is_deterministic() {
        let k1 = derive_shared_key(b"correct horse battery staple", "family").unwrap();
        let k2 = derive_shared_key(b"correct horse battery staple", "family").unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn shared_key_differs_by_vault_id() {
        let k1 = derive_shared_key(b"correct horse battery staple", "family").unwrap();
        let k2 = derive_shared_key(b"correct horse battery staple", "work").unwrap();
        assert_ne!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn shared_key_differs_by_passphrase() {
        let k1 = derive_shared_key(b"correct horse battery staple", "family").unwrap();
        let k2 = derive_shared_key(b"correct horse battery stapel", "family").unwrap();
        assert_ne!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn round_trip_record_encryption() {
        let key = derive_shared_key(b"hunter2", "test").unwrap();
        // Synthesize a record JSON.  We don't have a full
        // record constructor here, so just check that
        // encrypt -> decrypt returns the same bytes via
        // the encrypt_record/decrypt_record helpers.
        let rec = Record::Note(crate::records::Note {
            id: Uuid::new_v4(),
            title: "test".into(),
            body: "hello".into(),
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let payload = encrypt_record(&key, &rec).unwrap();
        let decoded = decrypt_record(&key, &payload).unwrap();
        match decoded {
            Record::Note(n) => {
                assert_eq!(n.title, "test");
                assert_eq!(n.body, "hello");
            }
            _ => panic!("wrong variant"),
        }
    }
}
