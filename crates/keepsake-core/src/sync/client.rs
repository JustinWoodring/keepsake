//! High-level sync client.  Walks the local vault, wraps each
//! record's on-disk envelope in an outer AEAD keyed by the
//! shared sync key, and pushes to the server.  On pull,
//! unwraps incoming payloads and applies them via the CRDT
//! layer (re-writing the inner envelope directly to the
//! `records` table — no re-encryption).
//!
//! The client talks to the vault-scoped server
//! (`/v1/vaults/:id/sync/{push,pull}`).  It does **not**
//! authenticate to the server — the server is a dumb
//! encrypted blob store, and knowing the URL + vault id is
//! the only access control.  The shared sync key is the only
//! thing protecting the server-side payload.
//!
//! ## Usage
//!
//! ```ignore
//! use keepsake_core::sync::client::SyncClient;
//!
//! let client = SyncClient::new("https://sync.example.com", "family");
//! // The session must have `shared_sync_keys` populated
//! // (i.e. shared sync must be set up for "family").
//! let n = client.push(&session).await?;
//! let n = client.pull(&mut session).await?;
//! # Ok::<(), keepsake_core::Error>(())
//! ```
//!
//! ## Wire payload format (nested envelope)
//!
//! For each `Change`, the `payload` is:
//!
//! ```text
//! outer_nonce:  24 bytes
//! outer_ct:     AEAD(
//!                 key:       shared_sync_key,
//!                 nonce:     outer_nonce,
//!                 aad:       "keepsake/sync/payload/v1",
//!                 plaintext:
//!                     aad_len (u32 LE)
//!                     || aead_nonce (24 bytes)
//!                     || aead_aad
//!                     || ciphertext)
//! ```
//!
//! The inner `aead_nonce`, `aead_aad`, and `ciphertext` are
//! the **exact** bytes stored in the local `records` table
//! for that record.  The inner AAD is the on-disk AAD built
//! by [`crate::vault::build_aad`] (variable-length because
//! it embeds the record `type` string), so we length-prefix
//! it.  The outer AEAD hides the length and structure of the
//! inner envelope from the server.
//!
//! See `docs/sync-protocol.md` for the full design.

use chrono::Utc;
use uuid::Uuid;

use crate::crypto::aead::{self, AeadKey, Nonce};
use crate::error::{Error, Result};
use crate::records::ALL_TYPES;
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
    /// Looks up the shared sync key in the session.  Each
    /// record is sent as a nested envelope: the inner is the
    /// on-disk `(aead_nonce, aead_aad, ciphertext)` triple
    /// sealed under the local vault key; the outer wraps it
    /// in an AEAD keyed by the shared sync key.
    pub async fn push(&self, session: &Session) -> Result<usize> {
        let shared_key = session
            .shared_sync_key(&self.vault_id)
            .ok_or_else(|| Error::Sync(format!(
                "no shared sync key for vault_id '{}' (set it up on the Sync page)",
                self.vault_id
            )))?;
        let mut changes: Vec<Change> = Vec::new();
        let mut lamport: u64 = 0;
        for t in ALL_TYPES {
            for h in session.vault.list_records(t)? {
                let (aead_nonce, aead_aad, ciphertext) = session
                    .vault
                    .get_record_envelope(h.id)?
                    .ok_or_else(|| Error::Sync(format!(
                        "record {} present in list but envelope missing",
                        h.id
                    )))?;
                let payload = wrap_envelope(shared_key, &aead_nonce, &aead_aad, &ciphertext)?;
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
    /// the number of changes applied.  For each change:
    /// peel the outer envelope, validate the inner against
    /// the local vault key, run the CRDT layer, and write
    /// the inner envelope directly to the `records` table
    /// (no re-encryption).
    pub async fn pull(&self, session: &mut Session) -> Result<usize> {
        let shared_key = session
            .shared_sync_key(&self.vault_id)
            .ok_or_else(|| Error::Sync(format!(
                "no shared sync key for vault_id '{}' (set it up on the Sync page)",
                self.vault_id
            )))?
            .clone();
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
            apply_remote_change(&shared_key, session, &ch)?;
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

/// Wrap an on-disk local envelope `(aead_nonce, aead_aad,
/// ciphertext)` in an outer AEAD keyed by `shared_key`.
/// Returns the wire payload: `outer_nonce || outer_ciphertext`.
fn wrap_envelope(
    shared_key: &AeadKey,
    aead_nonce: &[u8],
    aead_aad: &[u8],
    ciphertext: &[u8],
) -> Result<Vec<u8>> {
    if aead_nonce.len() != 24 {
        return Err(Error::Sync(format!(
            "inner nonce must be 24 bytes, got {}",
            aead_nonce.len()
        )));
    }
    let aad_len: u32 = aead_aad.len().try_into().map_err(|_| {
        Error::Sync(format!("inner aad too long: {}", aead_aad.len()))
    })?;
    let mut inner = Vec::with_capacity(4 + 24 + aead_aad.len() + ciphertext.len());
    inner.extend_from_slice(&aad_len.to_le_bytes());
    inner.extend_from_slice(aead_nonce);
    inner.extend_from_slice(aead_aad);
    inner.extend_from_slice(ciphertext);
    let outer_nonce = Nonce::random();
    let outer_ct = aead::encrypt(shared_key, &outer_nonce, &inner, PAYLOAD_AAD)?;
    let mut out = Vec::with_capacity(24 + outer_ct.len());
    out.extend_from_slice(outer_nonce.as_bytes());
    out.extend_from_slice(&outer_ct);
    Ok(out)
}

/// Unwrap a wire payload into the on-disk local envelope
/// `(aead_nonce, aead_aad, ciphertext)`.  Does NOT decrypt
/// the inner; the caller does that with the local vault
/// key.
fn unwrap_envelope(shared_key: &AeadKey, payload: &[u8]) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
    if payload.len() < 24 {
        return Err(Error::Sync("wire payload too short".into()));
    }
    let (outer_nonce_bytes, outer_ct) = payload.split_at(24);
    let outer_nonce_arr: [u8; 24] = outer_nonce_bytes.try_into().unwrap();
    let outer_nonce = Nonce::from_bytes(outer_nonce_arr);
    let inner = aead::decrypt(shared_key, &outer_nonce, outer_ct, PAYLOAD_AAD)?;
    if inner.len() < 4 + 24 {
        return Err(Error::Sync("inner envelope too short".into()));
    }
    let aad_len = u32::from_le_bytes(inner[0..4].try_into().unwrap()) as usize;
    if inner.len() < 4 + 24 + aad_len {
        return Err(Error::Sync("inner envelope truncated".into()));
    }
    let aead_nonce = inner[4..4 + 24].to_vec();
    let aead_aad = inner[4 + 24..4 + 24 + aad_len].to_vec();
    let ciphertext = inner[4 + 24 + aad_len..].to_vec();
    Ok((aead_nonce, aead_aad, ciphertext))
}

/// Apply a remote change to the local vault.  Unwraps the
/// outer envelope, validates the inner against the local
/// vault key, runs the CRDT layer, and writes the inner
/// envelope directly to the `records` table (no re-encryption).
fn apply_remote_change(shared_key: &AeadKey, session: &mut Session, ch: &Change) -> Result<()> {
    let record_id = ch.record_id.ok_or_else(|| {
        Error::Sync("change has no record_id; only record changes are supported in v1".into())
    })?;
    let (aead_nonce, aead_aad, ciphertext) = unwrap_envelope(shared_key, &ch.payload)?;

    // Build a RecordHeader from the inner AAD bytes.  The
    // AAD format is defined in `vault::build_aad`:
    //   "keepsake/record/v1\n" || type || 0x00 || schema_le || 0x00 || uuid
    let header = decode_inner_aad(&aead_aad, record_id, &ch.author, ch.ts)?;

    // Validate: decrypt the inner with the local vault key
    // to make sure it's actually for this vault.  This is
    // defense in depth — a forged payload from the server
    // will fail to peel the local layer.
    let local_key = session.vault.require_unlocked_for_sync()?;
    let plaintext = aead::decrypt(
        local_key,
        &Nonce::from_bytes(aead_nonce[..].try_into().unwrap()),
        &ciphertext,
        &aead_aad,
    )?;
    let rec: crate::records::Record = serde_json::from_slice(&plaintext)?;

    // Now run the CRDT layer with the local vault acting as
    // the local side, and the remote plaintext as the
    // remote side.  We use the raw envelope accessors so
    // the local vault never re-encrypts.
    let local_existed = session.vault.get_record_envelope(record_id)?.is_some();
    let local_read = || -> Result<Option<(crate::records::RecordHeader, crate::records::Record)>> {
        match session.vault.get_record(record_id) {
            Ok(pair) => Ok(Some(pair)),
            Err(Error::NotFound(_)) => Ok(None),
            Err(Error::Storage(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
            Err(e) => Err(e),
        }
    };
    let local_write = |h: &crate::records::RecordHeader,
                       _r: &crate::records::Record|
     -> Result<()> {
        // The header from CRDT is just metadata; the
        // payload bytes are the inner envelope from the
        // server, which we already have.  Build a fresh
        // header to point at the (now-stored) record.
        let new_header = crate::records::RecordHeader {
            r#type: h.r#type.clone(),
            schema_version: h.schema_version,
            id: h.id,
            created_by: h.created_by.clone(),
            updated_by: h.updated_by.clone(),
            created_at: h.created_at,
            updated_at: h.updated_at,
        };
        session.vault.put_record_envelope(
            new_header.id,
            &new_header.r#type,
            new_header.schema_version,
            &new_header.created_by,
            &new_header.updated_by,
            new_header.created_at,
            new_header.updated_at,
            &aead_nonce,
            &aead_aad,
            &ciphertext,
        )?;
        Ok(())
    };
    let outcome = crate::crdt::apply_remote_change(
        &header,
        &rec,
        local_read,
        local_write,
    )?;
    // outcome describes the merge; we only care that the
    // closure ran (which it did if we got here without
    // error).  Tally a meaningful "applied" count in the
    // outer caller.
    let _ = outcome;
    let _ = local_existed;
    Ok(())
}

/// Decode the inner AAD (built by `vault::build_aad`) into a
/// `RecordHeader`.  Validates the AAD bytes match the
/// expected layout and the `record_id` matches.
fn decode_inner_aad(
    aad: &[u8],
    record_id: Uuid,
    author: &str,
    ts: chrono::DateTime<Utc>,
) -> Result<crate::records::RecordHeader> {
    const PREFIX: &[u8] = b"keepsake/record/v1\n";
    if !aad.starts_with(PREFIX) {
        return Err(Error::Sync("inner AAD has wrong prefix".into()));
    }
    let mut rest = &aad[PREFIX.len()..];
    let nul = rest.iter().position(|&b| b == 0).ok_or_else(|| {
        Error::Sync("inner AAD missing type nul".into())
    })?;
    let r#type = std::str::from_utf8(&rest[..nul])
        .map_err(|_| Error::Sync("inner AAD type not utf-8".into()))?
        .to_string();
    rest = &rest[nul + 1..];
    if rest.len() < 4 + 1 + 16 {
        return Err(Error::Sync("inner AAD truncated".into()));
    }
    let schema_version = u32::from_le_bytes(rest[..4].try_into().unwrap());
    rest = &rest[4..];
    if rest[0] != 0 {
        return Err(Error::Sync("inner AAD missing schema nul".into()));
    }
    rest = &rest[1..];
    let id_bytes: [u8; 16] = rest[..16].try_into().unwrap();
    let id_in_aad = Uuid::from_bytes(id_bytes);
    if id_in_aad != record_id {
        return Err(Error::Sync("inner AAD record_id mismatch".into()));
    }
    Ok(crate::records::RecordHeader {
        r#type,
        schema_version,
        id: record_id,
        created_by: author.to_string(),
        updated_by: author.to_string(),
        created_at: ts,
        updated_at: ts,
    })
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
    fn wrap_unwrap_round_trip_preserves_inner_bytes() {
        let key = derive_shared_key(b"hunter2", "test").unwrap();
        let inner_nonce = [7u8; 24];
        let inner_aad = b"keepsake/record/v1\nnote\x00\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let inner_ct = b"some-ciphertext-bytes";
        let payload = wrap_envelope(&key, &inner_nonce, inner_aad, inner_ct).unwrap();
        // Outer layer adds a 24-byte nonce + AEAD tag (16 bytes),
        // so payload should be larger than the inner.
        assert!(payload.len() > 24 + inner_aad.len() + inner_ct.len());
        let (n, a, c) = unwrap_envelope(&key, &payload).unwrap();
        assert_eq!(n, inner_nonce);
        assert_eq!(a, inner_aad);
        assert_eq!(c, inner_ct);
    }

    #[test]
    fn unwrap_with_wrong_shared_key_fails() {
        let key1 = derive_shared_key(b"hunter2", "test").unwrap();
        let key2 = derive_shared_key(b"hunter3", "test").unwrap();
        let payload = wrap_envelope(&key1, &[0u8; 24], b"aad", b"ct").unwrap();
        assert!(unwrap_envelope(&key2, &payload).is_err());
    }

    #[test]
    fn unwrap_rejects_truncated_payload() {
        let key = derive_shared_key(b"k", "v").unwrap();
        assert!(unwrap_envelope(&key, &[0u8; 10]).is_err());
    }

    #[test]
    fn nested_envelope_round_trip_across_two_vaults() {
        // Simulate the full push/pull cycle between two
        // users of the same shared vault.  User A writes a
        // record locally, the sync client wraps the
        // envelope; user B unwraps and writes the (identical)
        // envelope into their local vault.  Both users can
        // read the record back.  The inner envelope is
        // byte-identical at both ends.
        use crate::crypto::aead::random_key;
        use crate::identity::VaultKey;
        use crate::records::Note;
        use crate::vault::Vault;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let path_a = dir.path().join("a.db");
        let path_b = dir.path().join("b.db");

        // Same vault key on both sides (the user-agnostic
        // shared vault key — every user of a vault has
        // access to this).
        let vk_bytes = *random_key().0;
        let vk = VaultKey::from_bytes(vk_bytes);

        let mut va = Vault::open_or_create(&path_a).unwrap();
        va.unlock(&vk).unwrap();
        let mut vb = Vault::open_or_create(&path_b).unwrap();
        vb.unlock(&vk).unwrap();

        // User A writes a record.  Read the inner envelope.
        let id = Uuid::new_v4();
        let now = Utc::now();
        let h = crate::records::RecordHeader {
            r#type: "note".into(),
            schema_version: 1,
            id,
            created_by: "alice".into(),
            updated_by: "alice".into(),
            created_at: now,
            updated_at: now,
        };
        let rec = crate::records::Record::Note(Note {
            id,
            title: "from A".into(),
            body: "hello".into(),
            tags: vec![],
            created_at: now,
            updated_at: now,
        });
        va.put_record(&h, &rec).unwrap();

        let (inner_n, inner_a, inner_c) = va
            .get_record_envelope(id)
            .unwrap()
            .expect("envelope present");

        // Wrap and unwrap (simulating push to a server, then
        // pull from the server).
        let shared = derive_shared_key(b"shared", "family").unwrap();
        let wire = wrap_envelope(&shared, &inner_n, &inner_a, &inner_c).unwrap();
        let (n2, a2, c2) = unwrap_envelope(&shared, &wire).unwrap();
        assert_eq!(n2, inner_n);
        assert_eq!(a2, inner_a);
        assert_eq!(c2, inner_c);

        // User B writes the inner envelope into their local
        // vault directly (no re-encryption).
        vb.put_record_envelope(
            id, "note", 1, "alice", "alice", now, now,
            &n2, &a2, &c2,
        ).unwrap();

        // User B reads the record back.  Body must match.
        let (_h, r2) = vb.get_record(id).unwrap();
        match r2 {
            crate::records::Record::Note(n) => {
                assert_eq!(n.title, "from A");
                assert_eq!(n.body, "hello");
            }
            _ => panic!("wrong variant"),
        }

        // The on-disk inner envelope at B must be
        // byte-identical to A's.
        let (bn, ba, bc) = vb
            .get_record_envelope(id)
            .unwrap()
            .expect("envelope present");
        assert_eq!(bn, inner_n);
        assert_eq!(ba, inner_a);
        assert_eq!(bc, inner_c);
    }
}
