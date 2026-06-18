//! Encrypted export bundle (`.ksk` file).
//!
//! Layout:
//!
//! ```text
//! header : "KSK1" (4) | u8 version | u8 kdf_id | u16 kdf_m_kib |
//!          u8 kdf_t | u8 kdf_p | [16] salt | u32 payload_len
//! payload: AEAD(export_key, ad="ksk/v1", plaintext)
//!          where the plaintext is:
//!            u32 vault_key_seal_len | sealed_vault_key_bytes |
//!            u32 record_count | repeated record_count times:
//!              u32 entry_len | u8 type | u8 schema_ver |
//!              u8 aead_nonce (24) | aead_ciphertext
//!            u32 attachment_count | (similar for attachments)
//!            u32 audit_count | (similar for audit entries)
//! ```
//!
//! The `export_key` is derived from a user-supplied passphrase via
//! Argon2id using the salt and parameters in the header.  This
//! passphrase can be (and usually is) different from the user's
//! daily password: it's the long, written-down string that
//! protects the export if it falls into the wrong hands.

use std::io::{Read, Write};
use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::crypto::{aead, hkdf, AeadKey, KdfParams, Nonce, KEY_LEN};
use crate::error::{Error, Result};
use crate::identity::{seal_vault_key, unseal_vault_key, MasterKey, SealedVaultKey, VaultKey};
use crate::records::{Record, RecordHeader};
use crate::vault::{AttachmentRow, SealedKeyRow, Vault};

/// Current `.ksk` format version.
pub const KSK_VERSION: u8 = 1;

/// Magic bytes at the start of a `.ksk` file.
pub const KSK_MAGIC: &[u8; 4] = b"KSK1";

/// A self-contained, encrypted export of a vault.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Bundle {
    /// Header (KDF params, salt, payload length).
    pub header: BundleHeader,
    /// AEAD ciphertext of the plaintext payload.
    pub payload: Vec<u8>,
    /// Nonce used for the payload.
    pub nonce: [u8; 24],
}

/// Header of a `.ksk` file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleHeader {
    /// Format version.
    pub version: u8,
    /// KDF id (0 = Argon2id).
    pub kdf_id: u8,
    /// KDF parameters.
    pub params: KdfParams,
    /// Argon2id salt (16 bytes).
    pub salt: [u8; 16],
    /// Length of the AEAD payload in bytes.
    pub payload_len: u32,
}

/// Plaintext payload (before AEAD).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Payload {
    /// Sealed vault key (a single-user seal; for multi-user
    /// exports we ship one bundle per user).
    pub sealed_vault_key: SealedVaultKeyBytes,
    /// Records to export.
    pub records: Vec<RecordEntry>,
    /// Attachments to export.
    pub attachments: Vec<AttachmentBytes>,
    /// Audit chain tail.
    pub audit: Vec<AuditBytes>,
    /// Wall-clock timestamp the bundle was created.
    pub created_at: DateTime<Utc>,
}

/// On-wire form of a sealed vault key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedVaultKeyBytes {
    /// AEAD nonce.
    pub nonce: [u8; 24],
    /// AEAD ciphertext.
    pub ciphertext: Vec<u8>,
}

/// A single record entry in the export.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordEntry {
    /// Header.
    pub header: RecordHeader,
    /// Decrypted record.
    pub record: Record,
}

/// A single attachment (encrypted ciphertext as stored on disk).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentBytes {
    pub id: uuid::Uuid,
    pub content_type: String,
    pub filename: String,
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
    pub plaintext_sha256: [u8; 32],
    pub created_at: DateTime<Utc>,
}

/// A single audit entry (already serialised to bytes by the caller).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditBytes {
    pub seq: u64,
    pub op: u8,
    pub actor: String,
    pub target_id: Option<String>,
    pub details: Option<String>,
    pub ts: DateTime<Utc>,
    pub prev_hash: [u8; 32],
    pub hash: [u8; 32],
}

impl From<crate::audit::AuditEntry> for AuditBytes {
    fn from(e: crate::audit::AuditEntry) -> Self {
        Self {
            seq: e.seq,
            op: e.op as u8,
            actor: e.actor,
            target_id: e.target_id,
            details: e.details,
            ts: e.ts,
            prev_hash: e.prev_hash,
            hash: e.hash,
        }
    }
}

impl From<AttachmentRow> for AttachmentBytes {
    fn from(r: AttachmentRow) -> Self {
        Self {
            id: r.id,
            content_type: r.content_type,
            filename: r.filename,
            nonce: r.nonce,
            ciphertext: r.ciphertext,
            plaintext_sha256: r.plaintext_sha256,
            created_at: r.created_at,
        }
    }
}

impl AttachmentBytes {
    /// Convert back to the vault row form.
    pub fn into_row(self) -> AttachmentRow {
        AttachmentRow {
            id: self.id,
            content_type: self.content_type,
            filename: self.filename,
            nonce: self.nonce,
            ciphertext: self.ciphertext,
            plaintext_sha256: self.plaintext_sha256,
            created_at: self.created_at,
        }
    }
}

impl SealedKeyRow {
    /// Convert into a `SealedVaultKeyBytes` for inclusion in a bundle.
    pub fn to_sealed_bytes(&self) -> SealedVaultKeyBytes {
        SealedVaultKeyBytes {
            nonce: self.seal_nonce,
            ciphertext: self.seal_ciphertext.clone(),
        }
    }
}

impl SealedVaultKeyBytes {
    /// Convert into a `SealedVaultKey` for the identity module.
    pub fn into_sealed(self) -> SealedVaultKey {
        SealedVaultKey {
            nonce: self.nonce,
            ciphertext: self.ciphertext,
        }
    }
}

/// Build a bundle from an unlocked vault.  Requires the master's
/// raw key and the per-user sealed row to re-seal the vault key
/// under the export passphrase.
pub fn build_bundle(
    vault: &Vault,
    master: &MasterKey,
    sealed: &SealedKeyRow,
    export_passphrase: &[u8],
) -> Result<Bundle> {
    if !vault.is_unlocked() {
        return Err(Error::Locked);
    }

    // Re-seal the vault key under the export passphrase.
    let export_salt = random_salt();
    let export_params = KdfParams::default();
    let export_master = crate::crypto::derive_master_key(
        export_passphrase,
        &export_salt,
        export_params,
    )?;

    // Pull records / attachments / audit.
    let mut records = Vec::new();
    for r#type in &[
        "login", "document", "identification",
        "insurance", "health", "bank_account", "credit_card",
        "investment", "income_source", "vehicle", "residence",
        "phone", "address", "contact", "subscription",
        "infrastructure", "domain", "runbook", "work_log", "note",
    ] {
        for h in vault.list_records(r#type)? {
            let (_h2, rec) = vault.get_record(h.id)?;
            records.push(RecordEntry { header: h, record: rec });
        }
    }

    // Attachments: we don't have a "list attachments" method on
    // Vault yet.  In v1 we skip attachments in the export; the
    // attachment API on `Vault` can be extended to enumerate
    // attachment ids and we'll plumb them in here.  For now the
    // payload's `attachments` field is empty.
    let attachments: Vec<AttachmentBytes> = Vec::new();

    // Audit chain.
    let audit: Vec<AuditBytes> = vault
        .read_audit()?
        .into_iter()
        .map(AuditBytes::from)
        .collect();

    // Seal the vault key with the export master and include the
    // sealed form.  We can't use the per-user sealed row directly
    // because that's the user's daily-password seal; we want a
    // passphrase-rewrap so the export is decoupled from the
    // daily password.
    let vault_key = unseal_vault_key(master, &SealedVaultKey {
        nonce: sealed.seal_nonce,
        ciphertext: sealed.seal_ciphertext.clone(),
    })?;
    let export_seal = seal_vault_key(&export_master, &vault_key)?;
    let payload = Payload {
        sealed_vault_key: SealedVaultKeyBytes {
            nonce: export_seal.nonce,
            ciphertext: export_seal.ciphertext,
        },
        records,
        attachments,
        audit,
        created_at: Utc::now(),
    };
    let payload_bytes = serde_json::to_vec(&payload)?;

    // Encrypt the payload with a key derived from the export master.
    let aead_key = {
        let sub_v = hkdf::derive_subkey(
            export_master.as_bytes(),
            &[],
            b"keepsake/ksk/v1",
            32,
        )?;
        let mut sub = [0u8; 32];
        sub.copy_from_slice(&sub_v);
        AeadKey::from_bytes(sub)
    };
    let nonce = Nonce::random();
    let payload_ct = aead::encrypt(&aead_key, &nonce, &payload_bytes, b"ksk/v1")?;
    // `payload_len` is the length of the *ciphertext* on disk
    // (plaintext + 16-byte tag).
    let payload_len = payload_ct.len() as u32;

    let header = BundleHeader {
        version: KSK_VERSION,
        kdf_id: 0,
        params: export_params,
        salt: export_salt,
        payload_len,
    };

    Ok(Bundle {
        header,
        payload: payload_ct,
        nonce: nonce.0,
    })
}

/// Write a bundle to disk.
pub fn write_bundle(bundle: &Bundle, path: &Path) -> Result<()> {
    let mut f = std::fs::File::create(path)?;
    f.write_all(KSK_MAGIC)?;
    f.write_all(&[bundle.header.version])?;
    f.write_all(&[bundle.header.kdf_id])?;
    f.write_all(&bundle.header.params.m_kib.to_le_bytes())?;
    f.write_all(&[bundle.header.params.t as u8])?;
    f.write_all(&[bundle.header.params.p as u8])?;
    f.write_all(&bundle.header.salt)?;
    f.write_all(&bundle.header.payload_len.to_le_bytes())?;
    f.write_all(&bundle.nonce)?;
    f.write_all(&bundle.payload)?;
    Ok(())
}

/// Read a bundle from disk.
pub fn read_bundle(path: &Path) -> Result<Bundle> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    parse_bundle(&buf)
}

/// Parse a bundle from raw bytes.
pub fn parse_bundle(buf: &[u8]) -> Result<Bundle> {
    if buf.len() < 4 + 1 + 1 + 4 + 1 + 1 + 16 + 4 + 24 {
        return Err(Error::Export("truncated header".into()));
    }
    if &buf[0..4] != KSK_MAGIC {
        return Err(Error::Export("bad magic".into()));
    }
    let mut off = 4;
    let version = buf[off]; off += 1;
    let kdf_id = buf[off]; off += 1;
    let mut m_kib = [0u8; 4];
    m_kib.copy_from_slice(&buf[off..off + 4]); off += 4;
    let t = buf[off] as u32; off += 1;
    let p = buf[off] as u32; off += 1;
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&buf[off..off + 16]); off += 16;
    let mut payload_len_bytes = [0u8; 4];
    payload_len_bytes.copy_from_slice(&buf[off..off + 4]); off += 4;
    let payload_len = u32::from_le_bytes(payload_len_bytes);
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&buf[off..off + 24]); off += 24;
    let payload = buf[off..].to_vec();
    if payload.len() as u64 != payload_len as u64 {
        return Err(Error::Export(format!(
            "payload length mismatch: header says {payload_len}, got {}",
            payload.len()
        )));
    }
    Ok(Bundle {
        header: BundleHeader {
            version,
            kdf_id,
            params: KdfParams { m_kib: u32::from_le_bytes(m_kib), t, p },
            salt,
            payload_len,
        },
        nonce,
        payload,
    })
}

/// Decrypt a bundle's payload, given the export passphrase.
/// Returns the plaintext [`Payload`] and the
/// [`MasterKey`] that was derived (callers can use this to
/// install the exported vault key on disk).
pub fn decrypt_bundle(bundle: &Bundle, export_passphrase: &[u8]) -> Result<(MasterKey, VaultKey, Payload)> {
    let export_master = crate::crypto::derive_master_key(
        export_passphrase,
        &bundle.header.salt,
        bundle.header.params,
    )?;
    let aead_key = {
        let sub_v = hkdf::derive_subkey(
            export_master.as_bytes(),
            &[],
            b"keepsake/ksk/v1",
            32,
        )?;
        let mut sub = [0u8; 32];
        sub.copy_from_slice(&sub_v);
        AeadKey::from_bytes(sub)
    };
    let nonce = Nonce::from_bytes(bundle.nonce);
    let payload_bytes = aead::decrypt(&aead_key, &nonce, &bundle.payload, b"ksk/v1")?;
    let payload: Payload = serde_json::from_slice(&payload_bytes)?;
    let sealed = SealedVaultKey {
        nonce: payload.sealed_vault_key.nonce,
        ciphertext: payload.sealed_vault_key.ciphertext.clone(),
    };
    let vault_key = unseal_vault_key(&export_master, &sealed)?;
    Ok((export_master, vault_key, payload))
}

fn random_salt() -> [u8; 16] {
    use rand::RngCore;
    let mut s = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut s);
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::aead::random_key;
    use crate::identity::{password_to_master_key, seal_vault_key, unseal_vault_key, EnvelopeKey};
    use crate::records::{Note, Record, RecordHeader};
    use tempfile::tempdir;

    fn fresh_vault(dir: &std::path::Path) -> (Vault, MasterKey, SealedKeyRow) {
        let path = dir.join("vault.db");
        let mut v = Vault::open_or_create(&path).unwrap();
        let params = KdfParams { m_kib: 8 * 1024, t: 1, p: 1 };
        let (mk, salt) = password_to_master_key(b"daily", params).unwrap();
        let vk = VaultKey::from_bytes(*random_key().0);
        v.unlock(&vk).unwrap();
        let sealed_vk = seal_vault_key(&mk, &vk).unwrap();
        let envelope = EnvelopeKey::from_master_key(&mk).unwrap();
        let row = SealedKeyRow {
            username: "justin".into(),
            device_id: [9u8; 16],
            kdf_salt: salt.0,
            kdf_params: params.encode(),
            seal_nonce: sealed_vk.nonce,
            seal_ciphertext: sealed_vk.ciphertext,
            envelope_pk: envelope.public_key().to_bytes(),
            created_at: Utc::now(),
        };
        v.put_sealed_key(&row).unwrap();
        // Add a record so the export has something to verify.
        let note = Note {
            id: uuid::Uuid::new_v4(),
            title: "Hi".into(),
            body: "body".into(),
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let record = Record::Note(note);
        let header = RecordHeader::new(&record, "justin");
        v.put_record(&header, &record).unwrap();
        (v, mk, row)
    }

    #[test]
    fn bundle_round_trip() {
        let dir = tempdir().unwrap();
        let (v, mk, row) = fresh_vault(dir.path());
        let bundle = build_bundle(&v, &mk, &row, b"export-pass").unwrap();
        let path = dir.path().join("out.ksk");
        write_bundle(&bundle, &path).unwrap();
        let bundle2 = read_bundle(&path).unwrap();
        let (_mk2, _vk2, payload) = decrypt_bundle(&bundle2, b"export-pass").unwrap();
        assert_eq!(payload.records.len(), 1);
        // Wrong passphrase fails.
        assert!(decrypt_bundle(&bundle2, b"wrong").is_err());
    }
}
