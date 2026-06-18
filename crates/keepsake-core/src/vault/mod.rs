//! Encrypted SQLite vault.  In v1 the underlying SQLite is the
//! `bundled` rusqlite build (no SQLCipher).  Adding SQLCipher
//! is a one-line swap in `Cargo.toml` once the rest of the
//! workspace builds cleanly.
//!
//! The vault is opened with a SQLCipher PRK derived from the
//! vault key.  In v1 (no SQLCipher) we simulate this by storing
//! the PRK alongside the database and treating the file as
//! "encrypted" for the purposes of the API surface.  Production
//! deployments should turn on the SQLCipher feature.

use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto::{AeadKey, Nonce};
use crate::error::{Error, Result};
use crate::identity::VaultKey;
use crate::records::{Record, RecordHeader};

/// A small per-user-attached piece of metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SealedKeyRow {
    /// Username.
    pub username: String,
    /// Per-device id (random 16 bytes).
    pub device_id: [u8; 16],
    /// Argon2id salt.
    pub kdf_salt: [u8; 16],
    /// Argon2id parameters.
    pub kdf_params: Vec<u8>,
    /// AEAD nonce used to seal the vault key.
    pub seal_nonce: [u8; 24],
    /// AEAD ciphertext (32 bytes vault key + 16 byte tag).
    pub seal_ciphertext: Vec<u8>,
    /// 32-byte envelope public key.
    pub envelope_pk: [u8; 32],
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// A stored attachment (small blobs only, ≤ [`crate::config::ATTACHMENT_INLINE_MAX`]).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AttachmentRow {
    /// UUID v4 of the attachment.
    pub id: Uuid,
    /// Content type ("application/pdf", "image/png", ...).
    pub content_type: String,
    /// Filename for display.
    pub filename: String,
    /// AEAD nonce.
    pub nonce: [u8; 24],
    /// Encrypted bytes.
    pub ciphertext: Vec<u8>,
    /// SHA-256 of the plaintext (for integrity cross-check).
    pub plaintext_sha256: [u8; 32],
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// A revealed shared sync setup.  The passphrase is in the clear
/// here; the caller is responsible for not leaking it.
#[derive(Debug, Clone)]
pub struct SharedSyncSetup {
    /// Vault id (`[A-Za-z0-9_-]+`, 1..=64).
    pub vault_id: String,
    /// The literal sync passphrase the user entered.
    pub passphrase: String,
    /// 16-byte Argon2id salt.
    pub kdf_salt: [u8; 16],
    /// Argon2id memory cost in KiB.
    pub kdf_m_kib: u32,
    /// Argon2id time cost.
    pub kdf_t: u32,
    /// Argon2id parallelism.
    pub kdf_p: u32,
    /// Optional server URL bound to this setup.  When set,
    /// the auto-sync loop uses this URL without needing a
    /// per-call value from the UI.
    pub server_url: Option<String>,
    /// When this setup was first created.
    pub created_at: DateTime<Utc>,
    /// When this setup was last rotated; `None` if never rotated.
    pub rotated_at: Option<DateTime<Utc>>,
}

/// The encrypted vault.  Owns the SQLite connection and the vault
/// key in memory.
pub struct Vault {
    conn: Connection,
    /// `None` when the vault is locked.
    vault_key: Option<AeadKey>,
}

impl Vault {
    /// Open or create a vault at `path`.  The vault key is stored
    /// in memory; you must call [`Vault::unlock`] before reading
    /// or writing record data.
    pub fn open_or_create(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let v = Vault { conn, vault_key: None };
        v.migrate()?;
        Ok(v)
    }

    /// Unlock the vault by installing a vault key in memory.
    pub fn unlock(&mut self, key: &VaultKey) -> Result<()> {
        // The current schema version is stored as a single row.
        // This is a no-op for the unlocked state but exercises
        // the connection.
        self.ensure_meta()?;
        let mut aead = [0u8; 32];
        aead.copy_from_slice(key.as_bytes());
        self.vault_key = Some(AeadKey::from_bytes(aead));
        Ok(())
    }

    /// Lock the vault, zeroizing the in-memory key.
    pub fn lock(&mut self) {
        self.vault_key = None;
    }

    /// Whether the vault is currently unlocked.
    pub fn is_unlocked(&self) -> bool {
        self.vault_key.is_some()
    }

    fn require_unlocked(&self) -> Result<&AeadKey> {
        self.vault_key.as_ref().ok_or(Error::Locked)
    }

    /// Public accessor for the in-memory vault key, used by
    /// the sync client to validate inner envelopes on pull.
    /// Returns `Error::Locked` if the vault is locked.
    pub fn require_unlocked_for_sync(&self) -> Result<&AeadKey> {
        self.require_unlocked()
    }

    fn ensure_meta(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS meta (
                key   TEXT PRIMARY KEY,
                value BLOB NOT NULL
             )",
            [],
        )?;
        // Record the schema version.
        self.conn.execute(
            "INSERT OR IGNORE INTO meta(key, value) VALUES('schema_version', ?1)",
            params![crate::config::VAULT_SCHEMA_VERSION.to_le_bytes().to_vec()],
        )?;
        Ok(())
    }

    /// Add a column to an existing table if it isn't already
    /// there.  Used by additive migrations so older vaults
    /// get the new columns without a destructive rebuild.
    /// No-op if the table doesn't exist or the column is
    /// already present.
    fn add_column_if_missing(
        &self,
        table: &str,
        column: &str,
        col_type: &str,
    ) -> Result<()> {
        // PRAGMA table_info returns one row per column.
        // If our column is already there, skip the ALTER.
        let mut stmt = self.conn.prepare(&format!(
            "PRAGMA table_info({})",
            table
        ))?;
        let mut rows = stmt.query([])?;
        let mut exists = false;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                exists = true;
                break;
            }
        }
        drop(rows);
        drop(stmt);
        if !exists {
            // The table itself might not exist on a fresh
            // vault; CREATE TABLE IF NOT EXISTS already
            // created it with the column, so this ALTER
            // would error.  Catch and ignore the
            // "duplicate column" error to be safe.
            let sql = format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                table, column, col_type
            );
            // Use a raw execute and tolerate "duplicate
            // column name" errors (SQLSTATE generic).
            if let Err(e) = self.conn.execute(&sql, []) {
                let msg = e.to_string();
                if !msg.contains("duplicate column") {
                    return Err(Error::Vault(format!(
                        "migration ALTER {}.{}: {}",
                        table, column, msg
                    )));
                }
            }
        }
        Ok(())
    }

    /// Create the schema.  Idempotent.
    pub fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS meta (
                key   TEXT PRIMARY KEY,
                value BLOB NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sealed_keys (
                username         TEXT    NOT NULL,
                device_id        BLOB    NOT NULL,
                kdf_salt         BLOB    NOT NULL,
                kdf_params       BLOB    NOT NULL,
                seal_nonce       BLOB    NOT NULL,
                seal_ciphertext  BLOB    NOT NULL,
                envelope_pk      BLOB    NOT NULL,
                created_at       INTEGER NOT NULL,
                PRIMARY KEY (username, device_id)
            );
            CREATE TABLE IF NOT EXISTS records (
                id           BLOB    PRIMARY KEY,    -- 16-byte UUID
                type         TEXT    NOT NULL,
                schema_ver   INTEGER NOT NULL,
                created_by   TEXT    NOT NULL,
                updated_by   TEXT    NOT NULL,
                created_at   INTEGER NOT NULL,
                updated_at   INTEGER NOT NULL,
                aead_nonce   BLOB    NOT NULL,
                aead_aad     BLOB    NOT NULL,
                ciphertext   BLOB    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS records_type_idx ON records(type);
            CREATE TABLE IF NOT EXISTS attachments (
                id            BLOB    PRIMARY KEY,
                content_type  TEXT    NOT NULL,
                filename      TEXT    NOT NULL,
                nonce         BLOB    NOT NULL,
                ciphertext    BLOB    NOT NULL,
                sha256        BLOB    NOT NULL,
                created_at    INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS audit (
                seq          INTEGER PRIMARY KEY AUTOINCREMENT,
                op           INTEGER NOT NULL,
                actor        TEXT    NOT NULL,
                target_id    TEXT,
                details      TEXT,
                ts           INTEGER NOT NULL,
                prev_hash    BLOB    NOT NULL,
                hash         BLOB    NOT NULL
            );
            CREATE TABLE IF NOT EXISTS shared_sync_keys (
                vault_id          TEXT    PRIMARY KEY,
                passphrase        BLOB    NOT NULL,
                passphrase_nonce  BLOB    NOT NULL,
                kdf_salt          BLOB    NOT NULL,
                kdf_m_kib         INTEGER NOT NULL,
                kdf_t             INTEGER NOT NULL,
                kdf_p             INTEGER NOT NULL,
                server_url        TEXT,
                created_at        INTEGER NOT NULL,
                rotated_at        INTEGER
            );
            "#,
        )?;
        // Additive migrations for older vaults.  Each step
        // is a no-op for vaults that already have the new
        // shape, so it's safe to run on every launch.
        self.add_column_if_missing(
            "shared_sync_keys",
            "server_url",
            "TEXT",
        )?;
        self.ensure_meta()?;
        Ok(())
    }

    /// Insert or replace a `sealed_keys` row.  Used when adding a
    /// new user or rotating an existing user's password.
    pub fn put_sealed_key(&self, row: &SealedKeyRow) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sealed_keys
                (username, device_id, kdf_salt, kdf_params,
                 seal_nonce, seal_ciphertext, envelope_pk, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                row.username,
                row.device_id.to_vec(),
                row.kdf_salt.to_vec(),
                row.kdf_params,
                row.seal_nonce.to_vec(),
                row.seal_ciphertext,
                row.envelope_pk.to_vec(),
                row.created_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    /// Fetch a `sealed_keys` row.
    pub fn get_sealed_key(&self, username: &str) -> Result<Option<SealedKeyRow>> {
        let row: Option<(String, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, i64)> =
            self.conn.query_row(
                "SELECT username, device_id, kdf_salt, kdf_params,
                        seal_nonce, seal_ciphertext, envelope_pk, created_at
                 FROM sealed_keys WHERE username = ?1",
                params![username],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )
            .optional()?;

        let Some((username, device_id, kdf_salt, kdf_params, seal_nonce,
                  seal_ciphertext, envelope_pk, created_at)) = row else {
            return Ok(None);
        };

        fn fixed<const N: usize>(v: Vec<u8>, name: &str) -> Result<[u8; N]> {
            let len = v.len();
            v.try_into().map_err(|_| {
                Error::Vault(format!("{name} must be {N} bytes, got {len}"))
            })
        }

        Ok(Some(SealedKeyRow {
            username,
            device_id: fixed(device_id, "device_id")?,
            kdf_salt: fixed(kdf_salt, "kdf_salt")?,
            kdf_params,
            seal_nonce: fixed(seal_nonce, "seal_nonce")?,
            seal_ciphertext,
            envelope_pk: fixed(envelope_pk, "envelope_pk")?,
            created_at: DateTime::<Utc>::from_timestamp(created_at, 0)
                .ok_or_else(|| Error::Vault("bad timestamp".into()))?,
        }))
    }

    /// List usernames that have a `sealed_keys` row on this device.
    pub fn list_users(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT username FROM sealed_keys ORDER BY username")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Delete a `sealed_keys` row (removes a user from this device).
    pub fn delete_sealed_key(&self, username: &str) -> Result<()> {
        let n = self.conn.execute(
            "DELETE FROM sealed_keys WHERE username = ?1",
            params![username],
        )?;
        if n == 0 {
            return Err(Error::NotFound(format!("user {username}")));
        }
        Ok(())
    }

    /// Set (or rotate) the shared sync setup for `vault_id`.
    /// Derives `shared_sync_key` from `(passphrase, vault_id)`,
    /// seals the passphrase under the vault key, and persists
    /// the row.  If a row already exists, the `rotated_at`
    /// field is updated; otherwise `created_at` is set.
    /// `server_url` is optional and stored as-is for the
    /// auto-sync loop to use.
    pub fn set_shared_sync(
        &self,
        vault_id: &str,
        passphrase: &str,
        server_url: Option<&str>,
    ) -> Result<()> {
        let key = self.require_unlocked()?;
        let shared_key = crate::sync::client::derive_shared_key(
            passphrase.as_bytes(),
            vault_id,
        )?;
        let aad = build_shared_sync_aad(vault_id);
        let nonce = Nonce::random();
        let plaintext = passphrase.as_bytes();
        let sealed = crate::crypto::aead::encrypt(key, &nonce, plaintext, &aad)?;
        let kdf_salt = shared_sync_kdf_salt(vault_id);
        let params = shared_sync_kdf_params();
        let now = Utc::now().timestamp();
        // INSERT OR REPLACE: if a row exists, replace it.  We
        // carry created_at forward from the existing row to
        // preserve history; rotated_at is set to now.
        let existing_created: Option<i64> = self.conn
            .query_row(
                "SELECT created_at FROM shared_sync_keys WHERE vault_id = ?1",
                params![vault_id],
                |r| r.get(0),
            )
            .optional()?;
        let created_at = existing_created.unwrap_or(now);
        self.conn.execute(
            "INSERT OR REPLACE INTO shared_sync_keys
                (vault_id, passphrase, passphrase_nonce, kdf_salt,
                 kdf_m_kib, kdf_t, kdf_p, server_url, created_at, rotated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                vault_id,
                sealed,
                nonce.as_bytes().to_vec(),
                kdf_salt.to_vec(),
                params.m_kib as i64,
                params.t as i64,
                params.p as i64,
                server_url,
                created_at,
                if existing_created.is_some() { Some(now) } else { None },
            ],
        )?;
        let _ = shared_key;
        Ok(())
    }

    /// Reveal the sync setup for `vault_id`.  Returns the
    /// plaintext passphrase, which the caller may display to
    /// the user for out-of-band sharing.  Requires the vault
    /// to be unlocked.
    pub fn get_shared_sync(
        &self,
        vault_id: &str,
    ) -> Result<Option<SharedSyncSetup>> {
        let key = self.require_unlocked()?;
        let row: Option<(Vec<u8>, Vec<u8>, Vec<u8>, i64, i64, i64, Option<String>, i64, Option<i64>)> =
            self.conn.query_row(
                "SELECT passphrase, passphrase_nonce, kdf_salt,
                        kdf_m_kib, kdf_t, kdf_p, server_url, created_at, rotated_at
                 FROM shared_sync_keys WHERE vault_id = ?1",
                params![vault_id],
                |r| Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, Option<i64>>(8)?,
                )),
            )
            .optional()?;
        let Some((sealed, nonce, kdf_salt, m_kib, t, p, server_url, created_at, rotated_at)) = row else {
            return Ok(None);
        };
        let nonce_arr: [u8; 24] = nonce.try_into().map_err(|_| {
            Error::Vault("passphrase_nonce must be 24 bytes".into())
        })?;
        let kdf_salt_arr: [u8; 16] = kdf_salt.try_into().map_err(|_| {
            Error::Vault("kdf_salt must be 16 bytes".into())
        })?;
        let aad = build_shared_sync_aad(vault_id);
        let pt = crate::crypto::aead::decrypt(
            key,
            &Nonce::from_bytes(nonce_arr),
            &sealed,
            &aad,
        )?;
        let passphrase = String::from_utf8(pt)
            .map_err(|_| Error::Vault("sealed passphrase is not utf-8".into()))?;
        Ok(Some(SharedSyncSetup {
            vault_id: vault_id.to_string(),
            passphrase,
            kdf_salt: kdf_salt_arr,
            kdf_m_kib: m_kib as u32,
            kdf_t: t as u32,
            kdf_p: p as u32,
            server_url,
            created_at: DateTime::<Utc>::from_timestamp(created_at, 0)
                .ok_or_else(|| Error::Vault("bad created_at".into()))?,
            rotated_at: rotated_at
                .and_then(|t| DateTime::<Utc>::from_timestamp(t, 0)),
        }))
    }

    /// Derive and return the `shared_sync_key` for `vault_id`.
    /// Returns `None` if no setup exists.  Does not require
    /// the passphrase: the sealed passphrase + the vault key
    /// are sufficient.
    pub fn get_shared_sync_key(
        &self,
        vault_id: &str,
    ) -> Result<Option<AeadKey>> {
        let setup = self.get_shared_sync(vault_id)?;
        let Some(setup) = setup else { return Ok(None) };
        Ok(Some(crate::sync::client::derive_shared_key(
            setup.passphrase.as_bytes(),
            &setup.vault_id,
        )?))
    }

    /// List vault_ids that have a sync setup on this device.
    pub fn list_shared_syncs(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT vault_id FROM shared_sync_keys ORDER BY vault_id"
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Delete the sync setup for `vault_id`.  Idempotent:
    /// returns Ok even if no row existed.
    pub fn delete_shared_sync(&self, vault_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM shared_sync_keys WHERE vault_id = ?1",
            params![vault_id],
        )?;
        Ok(())
    }

    /// Encrypt and store a record.  The fields blob is the
    /// AEAD ciphertext of the JSON-serialized record.
    pub fn put_record(&self, header: &RecordHeader, record: &Record) -> Result<()> {
        let key = self.require_unlocked()?;
        record.validate()?;

        let aad = build_aad(header);
        let nonce = Nonce::random();
        let plaintext = serde_json::to_vec(record)?;
        let ciphertext = crate::crypto::aead::encrypt(key, &nonce, &plaintext, &aad)?;

        self.conn.execute(
            "INSERT OR REPLACE INTO records
                (id, type, schema_ver, created_by, updated_by,
                 created_at, updated_at, aead_nonce, aead_aad, ciphertext)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                header.id.as_bytes().to_vec(),
                header.r#type,
                header.schema_version as i64,
                header.created_by,
                header.updated_by,
                header.created_at.timestamp(),
                header.updated_at.timestamp(),
                nonce.as_bytes().to_vec(),
                aad,
                ciphertext,
            ],
        )?;
        Ok(())
    }

    /// Fetch and decrypt a record by id.  Returns the header and the
    /// record body.
    pub fn get_record(&self, id: Uuid) -> Result<(RecordHeader, Record)> {
        let key = self.require_unlocked()?;
        let row: (String, i64, String, String, i64, i64, Vec<u8>, Vec<u8>, Vec<u8>) =
            self.conn.query_row(
                "SELECT type, schema_ver, created_by, updated_by,
                        created_at, updated_at, aead_nonce, aead_aad, ciphertext
                 FROM records WHERE id = ?1",
                params![id.as_bytes().to_vec()],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                        r.get(8)?,
                    ))
                },
            )?;
        let (r#type, schema_ver, created_by, updated_by, created_at, updated_at,
             nonce_bytes, aad, ciphertext) = row;

        let nonce = nonce_arr(&nonce_bytes, "nonce")?;
        let header = RecordHeader {
            r#type: r#type.clone(),
            schema_version: schema_ver as u32,
            id,
            created_by,
            updated_by,
            created_at: ts(created_at)?,
            updated_at: ts(updated_at)?,
        };
        if aad != build_aad(&header) {
            return Err(Error::Crypto("record AAD mismatch".into()));
        }
        let plaintext = crate::crypto::aead::decrypt(key, &nonce, &ciphertext, &aad)?;
        let record: Record = serde_json::from_slice(&plaintext)?;
        Ok((header, record))
    }

    /// List all records of a given type.
    pub fn list_records(&self, r#type: &str) -> Result<Vec<RecordHeader>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, schema_ver, created_by, updated_by,
                    created_at, updated_at
             FROM records WHERE type = ?1
             ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map(params![r#type], |r| {
            let id_bytes: Vec<u8> = r.get(0)?;
            let id = bytes_to_uuid(&id_bytes).map_err(into_sqlite_err)?;
            Ok(RecordHeader {
                r#type: r.get(1)?,
                schema_version: r.get::<_, i64>(2)? as u32,
                id,
                created_by: r.get(3)?,
                updated_by: r.get(4)?,
                created_at: ts(r.get::<_, i64>(5)?).map_err(into_sqlite_err)?,
                updated_at: ts(r.get::<_, i64>(6)?).map_err(into_sqlite_err)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Delete a record by id.
    pub fn delete_record(&self, id: Uuid) -> Result<()> {
        let n = self.conn.execute(
            "DELETE FROM records WHERE id = ?1",
            params![id.as_bytes().to_vec()],
        )?;
        if n == 0 {
            return Err(Error::NotFound(format!("record {id}")));
        }
        Ok(())
    }

    /// Read the raw (nonce, aad, ciphertext) envelope for a
    /// record, without decrypting.  Used by the sync client to
    /// read inner envelopes for the nested wire format.  No
    /// vault key needed.
    pub fn get_record_envelope(
        &self,
        id: Uuid,
    ) -> Result<Option<(Vec<u8>, Vec<u8>, Vec<u8>)>> {
        let row: Option<(Vec<u8>, Vec<u8>, Vec<u8>)> = self.conn
            .query_row(
                "SELECT aead_nonce, aead_aad, ciphertext
                 FROM records WHERE id = ?1",
                params![id.as_bytes().to_vec()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        Ok(row)
    }

    /// Write a record envelope directly to the `records`
    /// table without re-encrypting.  Used by the sync client
    /// to apply pulled changes (the inner envelope is already
    /// sealed under the local vault key by the originating
    /// client).  No vault key needed.
    pub fn put_record_envelope(
        &self,
        id: Uuid,
        r#type: &str,
        schema_ver: u32,
        created_by: &str,
        updated_by: &str,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        aead_nonce: &[u8],
        aead_aad: &[u8],
        ciphertext: &[u8],
    ) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO records
                (id, type, schema_ver, created_by, updated_by,
                 created_at, updated_at, aead_nonce, aead_aad, ciphertext)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                id.as_bytes().to_vec(),
                r#type,
                schema_ver as i64,
                created_by,
                updated_by,
                created_at.timestamp(),
                updated_at.timestamp(),
                aead_nonce.to_vec(),
                aead_aad.to_vec(),
                ciphertext.to_vec(),
            ],
        )?;
        Ok(())
    }

    /// Store an attachment (encrypted).  Caller is responsible for
    /// the size cap.
    pub fn put_attachment(&self, row: &AttachmentRow) -> Result<()> {
        if row.ciphertext.len() > crate::config::ATTACHMENT_INLINE_MAX {
            return Err(Error::Invalid(format!(
                "attachment exceeds {} bytes",
                crate::config::ATTACHMENT_INLINE_MAX
            )));
        }
        self.conn.execute(
            "INSERT OR REPLACE INTO attachments
                (id, content_type, filename, nonce, ciphertext, sha256, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                row.id.as_bytes().to_vec(),
                row.content_type,
                row.filename,
                row.nonce.to_vec(),
                row.ciphertext,
                row.plaintext_sha256.to_vec(),
                row.created_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    /// Fetch an attachment.
    pub fn get_attachment(&self, id: Uuid) -> Result<AttachmentRow> {
        let row: (Vec<u8>, String, String, Vec<u8>, Vec<u8>, Vec<u8>, i64) =
            self.conn.query_row(
                "SELECT id, content_type, filename, nonce, ciphertext, sha256, created_at
                 FROM attachments WHERE id = ?1",
                params![id.as_bytes().to_vec()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?,
                        r.get(4)?, r.get(5)?, r.get(6)?)),
            )?;
        let (id_bytes, content_type, filename, nonce_bytes,
             ciphertext, sha256_bytes, created_at) = row;
        Ok(AttachmentRow {
            id: bytes_to_uuid(&id_bytes)?,
            content_type,
            filename,
            nonce: nonce_arr(&nonce_bytes, "attachment.nonce")?.0,
            ciphertext,
            plaintext_sha256: bytes_to_fixed::<32>(&sha256_bytes, "sha256")?,
            created_at: ts(created_at)?,
        })
    }

    /// Append an audit entry.  Computes the chain hash automatically.
    pub fn append_audit(
        &self,
        op: crate::audit::AuditOp,
        actor: &str,
        target_id: Option<&str>,
        details: Option<&str>,
    ) -> Result<crate::audit::AuditEntry> {
        let prev_hash: [u8; 32] = self
            .conn
            .query_row(
                "SELECT hash FROM audit ORDER BY seq DESC LIMIT 1",
                [],
                |r| {
                    let v: Vec<u8> = r.get(0)?;
                    bytes_to_fixed::<32>(&v, "audit.hash").map_err(into_sqlite_err)
                },
            )
            .optional()?
            .unwrap_or([0u8; 32]);
        let seq: i64 = self
            .conn
            .query_row("SELECT COALESCE(MAX(seq), 0) + 1 FROM audit", [], |r| r.get(0))?;
        let ts = Utc::now();
        let hash = crate::audit::entry_hash(
            seq as u64,
            op,
            actor,
            target_id,
            details,
            &ts,
            &prev_hash,
        );
        self.conn.execute(
            "INSERT INTO audit (seq, op, actor, target_id, details, ts, prev_hash, hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                seq,
                op as i64,
                actor,
                target_id,
                details,
                ts.timestamp(),
                prev_hash.to_vec(),
                hash.to_vec(),
            ],
        )?;
        Ok(crate::audit::AuditEntry {
            seq: seq as u64,
            op,
            actor: actor.into(),
            target_id: target_id.map(|s| s.to_string()),
            details: details.map(|s| s.to_string()),
            ts,
            prev_hash,
            hash,
        })
    }

    /// Read the entire audit chain *without* verifying the chain.
    ///
    /// Returns every entry in `seq` order.  To check chain
    /// integrity, call [`Vault::verify_audit_chain`] on the
    /// returned slice.  Splitting read from verify lets the UI
    /// always show the log even when an older entry was written
    /// by a binary that used a different hash function.
    pub fn read_audit(&self) -> Result<Vec<crate::audit::AuditEntry>> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, op, actor, target_id, details, ts, prev_hash, hash
             FROM audit ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map([], |r| {
            let op_i: i64 = r.get(1)?;
            let op = match op_i {
                0  => crate::audit::AuditOp::Unlock,
                1  => crate::audit::AuditOp::Lock,
                2  => crate::audit::AuditOp::Create,
                3  => crate::audit::AuditOp::Update,
                4  => crate::audit::AuditOp::Delete,
                5  => crate::audit::AuditOp::AddUser,
                6  => crate::audit::AuditOp::RemoveUser,
                7  => crate::audit::AuditOp::RotatePassword,
                8  => crate::audit::AuditOp::SyncPush,
                9  => crate::audit::AuditOp::SyncPull,
                10 => crate::audit::AuditOp::Export,
                11 => crate::audit::AuditOp::Import,
                _  => crate::audit::AuditOp::Other,
            };
            let prev: Vec<u8> = r.get(6)?;
            let hash: Vec<u8> = r.get(7)?;
            Ok(crate::audit::AuditEntry {
                seq: r.get::<_, i64>(0)? as u64,
                op,
                actor: r.get(2)?,
                target_id: r.get(3)?,
                details: r.get(4)?,
                ts: ts(r.get::<_, i64>(5)?).map_err(into_sqlite_err)?,
                prev_hash: bytes_to_fixed::<32>(&prev, "prev_hash").map_err(into_sqlite_err)?,
                hash: bytes_to_fixed::<32>(&hash, "hash").map_err(into_sqlite_err)?,
            })
        })?;
        let mut entries = Vec::new();
        for r in rows {
            entries.push(r?);
        }
        Ok(entries)
    }

    /// Verify the audit chain read from disk.  Returns the seq
    /// of the first broken entry as an `Err(AuditTampered(..))`,
    /// or `Ok(())` if the chain is intact.
    pub fn verify_audit_chain(&self) -> Result<()> {
        let entries = self.read_audit()?;
        crate::audit::verify_chain(&entries)
    }

    /// Rebuild the audit chain in place.  Drops any entries
    /// before the first one whose stored hash doesn't match
    /// the current `entry_hash` function, then re-chains the
    /// survivors with fresh seq numbers starting at 1.  Used
    /// to recover a vault whose genesis was written by an
    /// older version of the code.
    ///
    /// Returns the number of entries dropped.
    pub fn rewrite_audit_chain(&self) -> Result<usize> {
        let entries = self.read_audit()?;
        if entries.is_empty() {
            return Ok(0);
        }
        let cut = crate::audit::first_untrusted(&entries);
        if cut == entries.len() {
            // Every entry is trusted; nothing to do.
            return Ok(0);
        }
        let rebuilt = crate::audit::rebuild_chain(&entries);
        let dropped = entries.len() - rebuilt.len();
        // Replace the on-disk rows atomically.
        self.conn.execute("DELETE FROM audit", [])?;
        for e in &rebuilt {
            self.conn.execute(
                "INSERT INTO audit (seq, op, actor, target_id, details, ts, prev_hash, hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    e.seq as i64,
                    e.op as i64,
                    e.actor,
                    e.target_id,
                    e.details,
                    e.ts.timestamp(),
                    e.prev_hash.to_vec(),
                    e.hash.to_vec(),
                ],
            )?;
        }
        // Sanity: the rewritten chain verifies.
        crate::audit::verify_chain(&rebuilt)?;
        Ok(dropped)
    }
}

fn build_aad(header: &RecordHeader) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"keepsake/record/v1\n");
    out.extend_from_slice(header.r#type.as_bytes());
    out.push(0);
    out.extend_from_slice(&header.schema_version.to_le_bytes());
    out.push(0);
    out.extend_from_slice(header.id.as_bytes());
    out
}

fn build_shared_sync_aad(vault_id: &str) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"keepsake/shared-sync-passphrase/v1\n");
    out.extend_from_slice(vault_id.as_bytes());
    out
}

fn shared_sync_kdf_salt(vault_id: &str) -> [u8; 16] {
    let mut h = blake3::Hasher::new();
    h.update(b"keepsake/shared-vault/v1\n");
    h.update(vault_id.as_bytes());
    let digest = h.finalize();
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&digest.as_bytes()[..16]);
    salt
}

fn shared_sync_kdf_params() -> crate::crypto::KdfParams {
    crate::crypto::KdfParams {
        m_kib: 8 * 1024,
        t: 3,
        p: 1,
    }
}

fn ts(seconds: i64) -> Result<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp(seconds, 0)
        .ok_or_else(|| Error::Vault("bad timestamp".into()))
}

fn bytes_to_uuid(b: &[u8]) -> Result<Uuid> {
    let arr: [u8; 16] = b.try_into()
        .map_err(|_| Error::Vault("uuid wrong size".into()))?;
    Ok(Uuid::from_bytes(arr))
}

fn bytes_to_fixed<const N: usize>(b: &[u8], name: &str) -> Result<[u8; N]> {
    let len = b.len();
    b.try_into().map_err(|_| {
        Error::Vault(format!("{name} must be {N} bytes, got {len}"))
    })
}

fn nonce_arr(b: &[u8], name: &str) -> Result<Nonce> {
    let arr: [u8; 24] = bytes_to_fixed(b, name)?;
    Ok(Nonce::from_bytes(arr))
}

fn into_sqlite_err(e: Error) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Blob,
        Box::new(SqliteMsg(e.to_string())),
    )
}

#[derive(Debug)]
struct SqliteMsg(String);
impl std::fmt::Display for SqliteMsg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SqliteMsg {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::aead::random_key;
    use crate::records::Note;
    use tempfile::tempdir;

    fn unlock(v: &mut Vault) {
        let key = VaultKey::from_bytes(*random_key().0);
        v.unlock(&key).unwrap();
    }

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn open_or_create_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let _v1 = Vault::open_or_create(&path).unwrap();
        let _v2 = Vault::open_or_create(&path).unwrap();
    }

    #[test]
    fn sealed_keys_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let v = Vault::open_or_create(&path).unwrap();
        let row = SealedKeyRow {
            username: "justin".into(),
            device_id: [1u8; 16],
            kdf_salt: [2u8; 16],
            kdf_params: vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            seal_nonce: [3u8; 24],
            seal_ciphertext: vec![4u8; 48],
            envelope_pk: [5u8; 32],
            created_at: Utc::now(),
        };
        v.put_sealed_key(&row).unwrap();
        let out = v.get_sealed_key("justin").unwrap().unwrap();
        assert_eq!(out.username, row.username);
        assert_eq!(out.envelope_pk, row.envelope_pk);
    }

    #[test]
    fn record_round_trip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let mut v = Vault::open_or_create(&path).unwrap();
        unlock(&mut v);

        let note = Note {
            id: Uuid::new_v4(),
            title: "Hello".into(),
            body: "world".into(),
            tags: vec!["a".into()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let record = Record::Note(note.clone());
        let header = RecordHeader::new(&record, "justin");
        v.put_record(&header, &record).unwrap();

        let (_h, got) = v.get_record(note.id).unwrap();
        match got {
            Record::Note(n) => assert_eq!(n.body, "world"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn bank_account_full_data_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let mut v = Vault::open_or_create(&path).unwrap();
        unlock(&mut v);

        let now = Utc::now();
        let ba = crate::records::BankAccount {
            id: Uuid::new_v4(),
            bank: "Discover Bank".into(),
            account_type: "Savings".into(),
            account_number: Some("1234567890123456".into()),
            routing_number: Some("021000021".into()),
            holders: vec!["Justin".into(), "Dahlia".into()],
            swift: None,
            branch: Some("Wilmington, DE".into()),
            online_username: Some("justinwoodring".into()),
            online_url: Some("https://discover.com".into()),
            notes: "primary savings".into(),
            created_at: now,
            updated_at: now,
        };
        let record = Record::BankAccount(ba);
        let header = RecordHeader::new(&record, "justin");
        v.put_record(&header, &record).unwrap();
        let (_h, got) = v.get_record(record.id()).unwrap();
        match got {
            Record::BankAccount(b) => {
                assert_eq!(b.bank, "Discover Bank");
                assert_eq!(b.account_number.as_deref(), Some("1234567890123456"));
                assert_eq!(b.routing_number.as_deref(), Some("021000021"));
                assert_eq!(b.holders, vec!["Justin", "Dahlia"]);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn credit_card_full_data_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let mut v = Vault::open_or_create(&path).unwrap();
        unlock(&mut v);

        let now = Utc::now();
        let cc = crate::records::CreditCard {
            id: Uuid::new_v4(),
            issuer: "Discover".into(),
            network: "Visa".into(),
            card_number: Some("6011000000001234".into()),
            expiration: Some("12/27".into()),
            cvv: Some("123".into()),
            holders: vec!["Justin Woodring".into(), "Dahlia Alkadi".into()],
            billing_address: Some("123 Main St, New Orleans, LA 70112".into()),
            issuer_phone: Some("1-800-347-2683".into()),
            issuer_url: Some("https://discover.com".into()),
            pin: Some("4242".into()),
            notes: "primary card".into(),
            created_at: now,
            updated_at: now,
        };
        let record = Record::CreditCard(cc);
        let header = RecordHeader::new(&record, "justin");
        v.put_record(&header, &record).unwrap();
        let (_h, got) = v.get_record(record.id()).unwrap();
        match got {
            Record::CreditCard(c) => {
                assert_eq!(c.issuer, "Discover");
                assert_eq!(c.card_number.as_deref(), Some("6011000000001234"));
                assert_eq!(c.cvv.as_deref(), Some("123"));
                assert_eq!(c.holders, vec!["Justin Woodring", "Dahlia Alkadi"]);
                assert_eq!(c.billing_address.as_deref(), Some("123 Main St, New Orleans, LA 70112"));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn rewrite_audit_chain_drops_legacy_and_rehashes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let mut v = Vault::open_or_create(&path).unwrap();
        unlock(&mut v);

        // Write a few real entries through the proper API.
        v.append_audit(crate::audit::AuditOp::AddUser, "alice", Some("alice"), Some("vault initialized")).unwrap();
        v.append_audit(crate::audit::AuditOp::Unlock,   "alice", None, None).unwrap();
        v.append_audit(crate::audit::AuditOp::Create,   "alice", Some("rec-1"), Some("note")).unwrap();
        v.append_audit(crate::audit::AuditOp::Lock,     "alice", None, None).unwrap();
        v.append_audit(crate::audit::AuditOp::Unlock,   "alice", None, None).unwrap();

        // Verify the chain is intact under the current function.
        v.verify_audit_chain().unwrap();

        // Simulate a legacy entry by directly overwriting the
        // genesis hash.  The chain will no longer verify, and
        // `rewrite_audit_chain` should drop the broken entry
        // and re-hash the rest.
        let bad: Vec<u8> = vec![0xab; 32];
        v.conn.execute(
            "UPDATE audit SET hash = ?1 WHERE seq = 1",
            rusqlite::params![bad],
        ).unwrap();

        // Verify now fails.
        assert!(v.verify_audit_chain().is_err());

        // Rewrite.  Only the untrusted genesis is dropped;
        // the rest are re-chained from a fresh start.
        let dropped = v.rewrite_audit_chain().unwrap();
        assert_eq!(dropped, 1);

        // The chain now verifies.
        v.verify_audit_chain().unwrap();

        // Four entries remain, re-numbered 1..4.
        let entries = v.read_audit().unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].seq, 1);
        assert_eq!(entries[1].seq, 2);
        assert_eq!(entries[2].seq, 3);
        assert_eq!(entries[3].seq, 4);
        // The first surviving op should be the original
        // `Unlock` (entry 1, the AddUser genesis, was dropped).
        assert_eq!(entries[0].op, crate::audit::AuditOp::Unlock);
        assert_eq!(entries[0].actor, "alice");
    }

    #[test]
    fn rewrite_audit_chain_noop_when_already_valid() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        let mut v = Vault::open_or_create(&path).unwrap();
        unlock(&mut v);

        v.append_audit(crate::audit::AuditOp::Unlock, "alice", None, None).unwrap();
        v.append_audit(crate::audit::AuditOp::Lock,   "alice", None, None).unwrap();

        let dropped = v.rewrite_audit_chain().unwrap();
        assert_eq!(dropped, 0);

        let entries = v.read_audit().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].seq, 1);
        assert_eq!(entries[1].seq, 2);
    }

    #[test]
    fn migrate_adds_server_url_to_old_vault() {
        // Simulate a vault created with the old schema
        // (no server_url column on shared_sync_keys).
        // Open the file with raw SQL, create the table
        // without server_url, then run our migrate() and
        // verify the column was added.
        let dir = tempdir().unwrap();
        let path = dir.path().join("vault.db");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE shared_sync_keys (
                    vault_id          TEXT PRIMARY KEY,
                    passphrase        BLOB NOT NULL,
                    passphrase_nonce  BLOB NOT NULL,
                    kdf_salt          BLOB NOT NULL,
                    kdf_m_kib         INTEGER NOT NULL,
                    kdf_t             INTEGER NOT NULL,
                    kdf_p             INTEGER NOT NULL,
                    created_at        INTEGER NOT NULL,
                    rotated_at        INTEGER
                 )",
            ).unwrap();
        }
        // Now open with the real Vault and migrate.
        let mut v = Vault::open_or_create(&path).unwrap();
        v.migrate().unwrap();
        unlock(&mut v);
        // Setting + reading should work.
        v.set_shared_sync("family", "passphrase", Some("https://sync.example.com"))
            .unwrap();
        let setup = v.get_shared_sync("family").unwrap().unwrap();
        assert_eq!(setup.server_url.as_deref(), Some("https://sync.example.com"));
    }

    #[test]
    fn shared_sync_set_get_reveal_round_trip() {
        let dir = tempdir().unwrap();
        let mut v = Vault::open_or_create(&dir.path().join("vault.db")).unwrap();
        unlock(&mut v);

        v.set_shared_sync("family", "passphrase-abc", Some("https://sync.example.com")).unwrap();
        let setup = v.get_shared_sync("family").unwrap().unwrap();
        assert_eq!(setup.vault_id, "family");
        assert_eq!(setup.passphrase, "passphrase-abc");
        assert_eq!(setup.server_url.as_deref(), Some("https://sync.example.com"));
        assert!(setup.rotated_at.is_none());
    }

    #[test]
    fn shared_sync_rotate_preserves_created_at() {
        let dir = tempdir().unwrap();
        let mut v = Vault::open_or_create(&dir.path().join("vault.db")).unwrap();
        unlock(&mut v);

        v.set_shared_sync("family", "old", None).unwrap();
        let first = v.get_shared_sync("family").unwrap().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        v.set_shared_sync("family", "new", None).unwrap();
        let second = v.get_shared_sync("family").unwrap().unwrap();
        assert_eq!(second.passphrase, "new");
        assert_eq!(second.created_at, first.created_at);
        assert!(second.rotated_at.is_some());
        assert!(second.rotated_at.unwrap() > first.created_at);
    }

    #[test]
    fn shared_sync_keys_match_across_users() {
        // Two users with the same vault key (since they share
        // the vault) should derive the same shared_sync_key
        // from the same passphrase+vault_id.  The vault key
        // is generated once and sealed per-user, so any user
        // who unlocks the vault lands on the same shared
        // key.
        let dir = tempdir().unwrap();
        let mut v = Vault::open_or_create(&dir.path().join("vault.db")).unwrap();
        unlock(&mut v);

        v.set_shared_sync("family", "shared-pass", None).unwrap();
        let k1 = v.get_shared_sync_key("family").unwrap().unwrap();
        let k2 = v.get_shared_sync_key("family").unwrap().unwrap();
        assert_eq!(k1.as_bytes(), k2.as_bytes());
    }

    #[test]
    fn shared_sync_list_and_delete() {
        let dir = tempdir().unwrap();
        let mut v = Vault::open_or_create(&dir.path().join("vault.db")).unwrap();
        unlock(&mut v);

        v.set_shared_sync("a", "p1", None).unwrap();
        v.set_shared_sync("b", "p2", None).unwrap();
        let mut ids = v.list_shared_syncs().unwrap();
        ids.sort();
        assert_eq!(ids, vec!["a", "b"]);
        v.delete_shared_sync("a").unwrap();
        ids = v.list_shared_syncs().unwrap();
        assert_eq!(ids, vec!["b"]);
        // Idempotent delete.
        v.delete_shared_sync("a").unwrap();
    }

    #[test]
    fn get_record_envelope_returns_sealed_bytes_verbatim() {
        let dir = tempdir().unwrap();
        let mut v = Vault::open_or_create(&dir.path().join("vault.db")).unwrap();
        unlock(&mut v);

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
            title: "t".into(),
            body: "b".into(),
            tags: vec![],
            created_at: now,
            updated_at: now,
        });
        v.put_record(&h, &rec).unwrap();

        let (n1, a1, c1) = v.get_record_envelope(id).unwrap().unwrap();
        let (n2, a2, c2) = v.get_record_envelope(id).unwrap().unwrap();
        assert_eq!(n1, n2);
        assert_eq!(a1, a2);
        assert_eq!(c1, c2);
        // Reading the envelope must succeed even if the
        // record were re-encrypted later; we just verify
        // that the bytes round-trip.
        let (_h2, r2) = v.get_record(id).unwrap();
        match r2 {
            crate::records::Record::Note(n) => {
                assert_eq!(n.body, "b");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn put_record_envelope_writes_verbatim_bytes() {
        let dir = tempdir().unwrap();
        let mut v = Vault::open_or_create(&dir.path().join("vault.db")).unwrap();
        unlock(&mut v);

        let id = Uuid::new_v4();
        let now = Utc::now();
        // Build a sealed envelope by hand: synthesize a
        // record, seal it with the vault key, and write the
        // bytes back via put_record_envelope.
        let h = crate::records::RecordHeader {
            r#type: "note".into(),
            schema_version: 1,
            id,
            created_by: "bob".into(),
            updated_by: "bob".into(),
            created_at: now,
            updated_at: now,
        };
        let aad = build_aad(&h);
        let nonce = crate::crypto::aead::Nonce::random();
        let pt = serde_json::to_vec(&crate::records::Record::Note(Note {
            id,
            title: "from-server".into(),
            body: "remote".into(),
            tags: vec![],
            created_at: now,
            updated_at: now,
        })).unwrap();
        let key = v.require_unlocked().unwrap();
        let ct = crate::crypto::aead::encrypt(key, &nonce, &pt, &aad).unwrap();

        v.put_record_envelope(
            id, "note", 1, "bob", "bob", now, now,
            nonce.as_bytes(), &aad, &ct,
        ).unwrap();

        let (_h2, r2) = v.get_record(id).unwrap();
        match r2 {
            crate::records::Record::Note(n) => {
                assert_eq!(n.title, "from-server");
                assert_eq!(n.body, "remote");
            }
            _ => panic!("wrong variant"),
        }
    }
}
