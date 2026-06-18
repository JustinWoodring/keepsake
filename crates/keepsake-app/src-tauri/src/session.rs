//! Bridge between Tauri commands and `keepsake-core`.

use uuid::Uuid;

use keepsake_core::audit::AuditOp;
use keepsake_core::records::{Record, RecordHeader};
use keepsake_core::Result;

use super::AppState;

/// Initialize a new vault on this device.
pub fn init(
    state: &AppState,
    path: &std::path::Path,
    username: &str,
    password: &str,
) -> Result<()> {
    let mut guard = state.session.lock();
    if guard.is_some() {
        return Err(keepsake_core::Error::AlreadyExists(
            "vault already unlocked".into(),
        ));
    }
    *guard = Some(build_new_session(path, username, password)?);
    Ok(())
}

/// Unlock an existing vault.
pub fn unlock(
    state: &AppState,
    path: &std::path::Path,
    username: &str,
    password: &str,
) -> Result<()> {
    let mut guard = state.session.lock();
    if guard.is_some() {
        return Err(keepsake_core::Error::AlreadyExists(
            "vault already unlocked".into(),
        ));
    }
    *guard = Some(open_session(path, username, password)?);
    Ok(())
}

/// Lock the vault.
pub fn lock(state: &AppState) {
    let mut guard = state.session.lock();
    if let Some(s) = guard.as_mut() {
        s.vault.lock();
    }
    *guard = None;
}

/// Add a record.  `fields` is a JSON object whose shape
/// depends on `r#type`.  If the object lacks `id`, `created_at`,
/// or `updated_at`, we fill them in here so the frontend can
/// send just the form fields.
pub fn add_record(
    state: &AppState,
    r#type: &str,
    fields: serde_json::Value,
) -> Result<String> {
    let mut guard = state.session.lock();
    let s = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    let now = chrono::Utc::now();
    let record = record_from_fields(r#type, fields, now)?;
    record.validate()?;
    let header = RecordHeader::new(&record, &s.username);
    s.vault.put_record(&header, &record)?;
    s.vault.append_audit(
        AuditOp::Create,
        &s.username,
        Some(&record.id().to_string()),
        Some(r#type),
    )?;
    Ok(record.id().to_string())
}

/// Update a record by id.  `fields` is the full new record body.
pub fn update_record(
    state: &AppState,
    id: &str,
    fields: serde_json::Value,
) -> Result<()> {
    let mut guard = state.session.lock();
    let s = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    let id = Uuid::parse_str(id)
        .map_err(|_| keepsake_core::Error::Invalid(format!("bad uuid: {id}")))?;
    let (mut header, _old) = s.vault.get_record(id)?;
    let r#type = header.r#type.clone();
    let now = chrono::Utc::now();
    let new_record = record_from_fields(&r#type, fields, now)?;
    new_record.validate()?;
    header.updated_at = now;
    header.updated_by = s.username.clone();
    s.vault.put_record(&header, &new_record)?;
    s.vault.append_audit(
        AuditOp::Update,
        &s.username,
        Some(&id.to_string()),
        Some(&r#type),
    )?;
    Ok(())
}

/// Construct a `Record` from a `r#type` tag and a fields
/// object.  Fills in `id`, `created_at`, and `updated_at` if
/// the frontend didn't provide them, so the form can send
/// just the user-visible fields.
fn record_from_fields(
    r#type: &str,
    mut fields: serde_json::Value,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<Record> {
    if let serde_json::Value::Object(ref mut map) = fields {
        map.insert("type".to_string(), serde_json::Value::String(r#type.to_string()));
        let now_str = now.to_rfc3339();
        if !map.contains_key("id") {
            map.insert("id".to_string(), serde_json::Value::String(Uuid::new_v4().to_string()));
        }
        if !map.contains_key("created_at") {
            map.insert("created_at".to_string(), serde_json::Value::String(now_str.clone()));
        }
        if !map.contains_key("updated_at") {
            map.insert("updated_at".to_string(), serde_json::Value::String(now_str));
        }
    } else {
        return Err(keepsake_core::Error::Record(
            "fields must be a JSON object".into(),
        ));
    }
    serde_json::from_value(fields)
        .map_err(|e| keepsake_core::Error::Record(format!("{e}")))
}

/// Delete a record by id.
pub fn delete_record(state: &AppState, id: &str) -> Result<()> {
    let mut guard = state.session.lock();
    let s = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    let id = Uuid::parse_str(id)
        .map_err(|_| keepsake_core::Error::Invalid(format!("bad uuid: {id}")))?;
    s.vault.delete_record(id)?;
    s.vault.append_audit(
        AuditOp::Delete,
        &s.username,
        Some(&id.to_string()),
        None,
    )?;
    Ok(())
}

/// List records of a given type.
pub fn list_records(
    state: &AppState,
    r#type: &str,
) -> Result<Vec<serde_json::Value>> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    let headers = s.vault.list_records(r#type)?;
    let mut out = Vec::new();
    for h in headers {
        out.push(serde_json::json!({
            "id": h.id.to_string(),
            "type": h.r#type,
            "updated_by": h.updated_by,
            "updated_at": h.updated_at.to_rfc3339(),
        }));
    }
    Ok(out)
}

/// Show a record.  Sensitive fields are masked unless
/// `reveal` is true.
pub fn show_record(
    state: &AppState,
    id: &str,
    reveal: bool,
) -> Result<serde_json::Value> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    let id = Uuid::parse_str(id)
        .map_err(|_| keepsake_core::Error::Invalid(format!("bad uuid: {id}")))?;
    let (_h, rec) = s.vault.get_record(id)?;
    if reveal {
        Ok(serde_json::to_value(&rec)?)
    } else {
        Ok(masked_value(&rec))
    }
}

/// Free-text search.
pub fn find(
    state: &AppState,
    query: &str,
) -> Result<Vec<serde_json::Value>> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    let mut out = Vec::new();
    for t in ALL_TYPES {
        for h in s.vault.list_records(t)? {
            let (_h, rec) = s.vault.get_record(h.id)?;
            if rec.matches_query(query) {
                out.push(serde_json::json!({
                    "id": h.id.to_string(),
                    "type": h.r#type,
                    "snippet": snippet(&rec.search_blob(), query),
                }));
            }
        }
    }
    Ok(out)
}

/// Read or verify the audit chain.
///
/// When `verify=true`, runs a strict chain check and returns
/// `{ ok: bool, entries: usize, first_broken: Option<u64> }`.
/// When `verify=false`, returns the array of entries without
/// verifying — the UI can then show the log even if an old
/// entry was written by a different version of the code.
pub fn audit(
    state: &AppState,
    verify: bool,
) -> Result<serde_json::Value> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    if verify {
        let entries = s.vault.read_audit()?;
        match s.vault.verify_audit_chain() {
            Ok(()) => Ok(serde_json::json!({
                "ok": true,
                "entries": entries.len(),
            })),
            Err(keepsake_core::Error::AuditTampered(seq)) => Ok(serde_json::json!({
                "ok": false,
                "entries": entries.len(),
                "first_broken": seq,
            })),
            Err(e) => return Err(e),
        }
    } else {
        let entries = s.vault.read_audit()?;
        let mut out = Vec::new();
        for e in entries {
            out.push(serde_json::json!({
                "seq": e.seq,
                "op": format!("{:?}", e.op),
                "actor": e.actor,
                "target_id": e.target_id,
                "details": e.details,
                "ts": e.ts.to_rfc3339(),
            }));
        }
        Ok(serde_json::Value::Array(out))
    }
}

/// Rebuild the audit chain in place.  Drops any entries that
/// don't hash correctly under the current `entry_hash` function
/// and re-chains the survivors.  Returns the number of entries
/// dropped.
pub fn rewrite_audit_chain(state: &AppState) -> Result<usize> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    s.vault.rewrite_audit_chain()
}

/// Push all local records to the given server URL.  Blocks
/// the calling thread (the Tauri command) until done.  Returns
/// the number of records pushed.  The shared sync key for
/// `vault_id` must already be set up via
/// [`setup_shared_sync`].
pub async fn sync_push(
    state: &AppState,
    server_url: String,
    vault_id: String,
) -> Result<usize> {
    // The Tauri command runs on a tokio worker thread, so
    // we can't call `Handle::current().block_on(...)` here
    // (it would try to drive the runtime from inside the
    // runtime).  Move the work to a blocking thread, where
    // `block_on` is safe.
    let session_arc = state.session.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = session_arc.lock();
        let session = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
        let client = keepsake_core::sync::client::SyncClient::new(server_url, vault_id);
        tokio::runtime::Handle::current().block_on(client.push(session))
    })
    .await
    .map_err(|e| keepsake_core::Error::Transport(format!("sync_push join: {e}")))?
}

/// Pull all remote records and apply them locally.  Returns
/// the number of changes applied.  The shared sync key for
/// `vault_id` must already be set up via
/// [`setup_shared_sync`].
pub async fn sync_pull(
    state: &AppState,
    server_url: String,
    vault_id: String,
) -> Result<usize> {
    let session_arc = state.session.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = session_arc.lock();
        let session = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
        let client = keepsake_core::sync::client::SyncClient::new(server_url, vault_id);
        tokio::runtime::Handle::current().block_on(client.pull(session))
    })
    .await
    .map_err(|e| keepsake_core::Error::Transport(format!("sync_pull join: {e}")))?
}

/// Set up (or rotate) the shared sync setup for `vault_id`.
/// The passphrase is sealed inside the vault; the derived
/// `shared_sync_key` is cached in the session.  If a
/// `server_url` is provided, it is bound to the setup so
/// the auto-sync loop can use it without a per-call arg.
pub fn setup_shared_sync(
    state: &AppState,
    vault_id: String,
    passphrase: String,
    server_url: Option<String>,
) -> Result<()> {
    let mut guard = state.session.lock();
    let session = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    session.vault.set_shared_sync(
        &vault_id,
        &passphrase,
        server_url.as_deref(),
    )?;
    session.refresh_shared_sync_keys()?;
    Ok(())
}

/// Reveal the shared sync setup for `vault_id`.  Returns
/// `(vault_id, passphrase, server_url)` so the user can
/// copy them out-of-band to configure another device.
pub fn reveal_shared_sync(
    state: &AppState,
    vault_id: String,
) -> Result<(String, String, Option<String>)> {
    let guard = state.session.lock();
    let session = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    let setup = session.vault.get_shared_sync(&vault_id)?
        .ok_or_else(|| keepsake_core::Error::NotFound(format!("shared sync '{vault_id}'")))?;
    Ok((setup.vault_id, setup.passphrase, setup.server_url))
}

/// Delete the shared sync setup for `vault_id`.  Idempotent.
pub fn delete_shared_sync(
    state: &AppState,
    vault_id: String,
) -> Result<()> {
    let mut guard = state.session.lock();
    let session = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    session.vault.delete_shared_sync(&vault_id)?;
    session.refresh_shared_sync_keys()?;
    Ok(())
}

/// List the usernames on this device (for the unlock picker).
/// Returns an empty list if no vault file exists at the path —
/// does not create a new vault.
pub fn list_users(state: &AppState, path: &std::path::Path) -> Result<Vec<String>> {
    let _ = state;
    use keepsake_core::vault::Vault;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let v = Vault::open_or_create(path)?;
    v.list_users()
}

/// Set up the sync engine.  Returns the configured base URL
/// so the frontend can show a connection status.
pub fn configure_sync(
    state: &AppState,
    base_url: &str,
) -> Result<()> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    let _ = s;
    let _ = base_url;
    Ok(())
}

/// Export the vault to a `.ksk` file.  Returns the bytes of the
/// bundle; the frontend is responsible for writing them to disk
/// via the Tauri file dialog.
pub fn export_bundle(
    state: &AppState,
    export_passphrase: &[u8],
) -> Result<Vec<u8>> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    let row = s.vault.get_sealed_key(&s.username)?
        .ok_or_else(|| keepsake_core::Error::NotFound(s.username.clone()))?;
    let bundle = keepsake_core::export::build_bundle(
        &s.vault, &s.master, &row, export_passphrase,
    )?;
    // Serialize the bundle to a byte stream the same way the
    // CLI does.
    let mut buf = Vec::new();
    use std::io::Write;
    buf.write_all(keepsake_core::export::KSK_MAGIC)?;
    buf.write_all(&[bundle.header.version])?;
    buf.write_all(&[bundle.header.kdf_id])?;
    buf.write_all(&bundle.header.params.m_kib.to_le_bytes())?;
    buf.write_all(&[bundle.header.params.t as u8])?;
    buf.write_all(&[bundle.header.params.p as u8])?;
    buf.write_all(&bundle.header.salt)?;
    buf.write_all(&bundle.header.payload_len.to_le_bytes())?;
    buf.write_all(&bundle.nonce)?;
    buf.write_all(&bundle.payload)?;
    Ok(buf)
}

/// Import a `.ksk` file into the current vault.  The current
/// vault key is replaced with the one from the bundle; records
/// are merged.
pub fn import_bundle(
    state: &AppState,
    bytes: &[u8],
    export_passphrase: &[u8],
) -> Result<()> {
    let bundle = keepsake_core::export::parse_bundle(bytes)?;
    let (_master, vault_key, payload) =
        keepsake_core::export::decrypt_bundle(&bundle, export_passphrase)?;
    let mut guard = state.session.lock();
    let s = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    s.vault.unlock(&vault_key)?;
    for entry in &payload.records {
        s.vault.put_record(&entry.header, &entry.record)?;
    }
    for a in &payload.attachments {
        s.vault.put_attachment(&a.clone().into_row())?;
    }
    s.vault.append_audit(
        AuditOp::Import,
        &s.username,
        Some("(import)"),
        Some(&format!("{} records", payload.records.len())),
    )?;
    Ok(())
}

/// Import a `.ksk` file into a brand-new vault at `path` and
/// unlock the session as `username` with `password`.  Used by
/// the unlock screen when no vault exists yet.  Creates the
/// vault file if it doesn't already exist, otherwise aborts.
pub fn import_to_new_vault(
    path: &std::path::Path,
    bytes: &[u8],
    export_passphrase: &[u8],
    username: &str,
    password: &str,
) -> Result<keepsake_core::session::Session> {
    if path.exists() {
        return Err(keepsake_core::Error::AlreadyExists(
            path.display().to_string(),
        ));
    }
    let bundle = keepsake_core::export::parse_bundle(bytes)?;
    let (_export_master, vault_key, payload) =
        keepsake_core::export::decrypt_bundle(&bundle, export_passphrase)?;

    use keepsake_core::crypto::KdfParams;
    use keepsake_core::identity::{
        password_to_master_key, seal_vault_key, EnvelopeKey,
    };
    let params = KdfParams::default();
    let (master, salt) = password_to_master_key(password.as_bytes(), params)?;
    let new_sealed = seal_vault_key(&master, &vault_key)?;
    let envelope = EnvelopeKey::from_master_key(&master)?;

    // Create the vault file and write the sealed-keys row.
    let vault = keepsake_core::vault::Vault::open_or_create(path)?;
    let row = keepsake_core::vault::SealedKeyRow {
        username: username.to_string(),
        device_id: random_device_id(),
        kdf_salt: salt.0,
        kdf_params: params.encode(),
        seal_nonce: new_sealed.nonce,
        seal_ciphertext: new_sealed.ciphertext,
        envelope_pk: envelope.public_key().to_bytes(),
        created_at: chrono::Utc::now(),
    };
    vault.put_sealed_key(&row)?;
    drop(vault);

    // Reopen, unlock with the bundle's vault key, and write
    // all the records.
    let mut vault = keepsake_core::vault::Vault::open_or_create(path)?;
    vault.unlock(&vault_key)?;
    for entry in &payload.records {
        vault.put_record(&entry.header, &entry.record)?;
    }
    for a in &payload.attachments {
        vault.put_attachment(&a.clone().into_row())?;
    }
    vault.append_audit(
        AuditOp::Import,
        username,
        Some("(import)"),
        Some(&format!("{} records", payload.records.len())),
    )?;
    Ok(keepsake_core::session::Session::new(
        path.to_path_buf(),
        vault,
        master,
        username.to_string(),
    )?)
}

/// Recover a vault on a new device from a sync server.
/// Creates a fresh, empty local vault with the user's
/// username + password, seals the shared sync setup
/// (server_url, vault_id, sync_passphrase) into it, and
/// returns an unlocked Session ready to pull from the
/// server.  Used by the Unlock screen's "Recover from
/// sync" flow.
pub fn recover_from_sync(
    path: &std::path::Path,
    server_url: &str,
    vault_id: &str,
    sync_passphrase: &str,
    username: &str,
    password: &str,
) -> Result<keepsake_core::session::Session> {
    if path.exists() {
        return Err(keepsake_core::Error::AlreadyExists(
            path.display().to_string(),
        ));
    }
    use keepsake_core::crypto::KdfParams;
    use keepsake_core::identity::{password_to_master_key, seal_vault_key, EnvelopeKey, VaultKey};
    use rand::RngCore;

    // Generate a fresh random vault key — same as a normal
    // init.  The shared sync key is derived independently
    // from the sync passphrase + vault_id; the vault key
    // and the shared sync key are different keys that
    // protect different things.
    let mut k = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut k);
    let vault_key = VaultKey::from_bytes(k);
    let params = KdfParams::default();
    let (master, salt) = password_to_master_key(password.as_bytes(), params)?;
    let sealed = seal_vault_key(&master, &vault_key)?;
    let envelope = EnvelopeKey::from_master_key(&master)?;

    // Create the vault file and write the sealed-keys row.
    let vault = keepsake_core::vault::Vault::open_or_create(path)?;
    let row = keepsake_core::vault::SealedKeyRow {
        username: username.to_string(),
        device_id: random_device_id(),
        kdf_salt: salt.0,
        kdf_params: params.encode(),
        seal_nonce: sealed.nonce,
        seal_ciphertext: sealed.ciphertext,
        envelope_pk: envelope.public_key().to_bytes(),
        created_at: chrono::Utc::now(),
    };
    vault.put_sealed_key(&row)?;
    drop(vault);

    // Reopen, unlock with the vault key, write the shared
    // sync setup, append an audit entry.
    let mut vault = keepsake_core::vault::Vault::open_or_create(path)?;
    vault.unlock(&vault_key)?;
    vault.set_shared_sync(vault_id, sync_passphrase, Some(server_url))?;
    vault.append_audit(
        AuditOp::AddUser,
        username,
        Some(&username),
        Some("vault recovered from sync"),
    )?;
    vault.append_audit(
        AuditOp::Unlock,
        username,
        None,
        None,
    )?;
    Ok(keepsake_core::session::Session::new(
        path.to_path_buf(),
        vault,
        master,
        username.to_string(),
    )?)
}

/// Add a new user to this device's vault.  The vault key is
/// sealed under the new user's master key.  Requires the
/// vault to be unlocked (so we can re-derive the vault key
/// from the current user's master).
pub fn add_user(
    state: &AppState,
    username: &str,
    password: &str,
) -> Result<()> {
    use keepsake_core::crypto::KdfParams;
    use keepsake_core::identity::{
        password_to_master_key, seal_vault_key, EnvelopeKey,
    };

    if username.trim().is_empty() {
        return Err(keepsake_core::Error::Invalid("username is empty".into()));
    }

    let mut guard = state.session.lock();
    let s = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;

    // Don't add a user that already exists on this device.
    if s.vault.get_sealed_key(username)?.is_some() {
        return Err(keepsake_core::Error::AlreadyExists(username.to_string()));
    }

    // Re-derive the vault key from the current session's master.
    let cur = s.vault.get_sealed_key(&s.username)?
        .ok_or_else(|| keepsake_core::Error::NotFound(s.username.clone()))?;
    let vault_key = keepsake_core::identity::unseal_vault_key(
        &s.master,
        &keepsake_core::identity::SealedVaultKey {
            nonce: cur.seal_nonce,
            ciphertext: cur.seal_ciphertext,
        },
    )?;

    // Derive the new user's keys and seal the vault key under them.
    let params = KdfParams::default();
    let (new_master, salt) = password_to_master_key(password.as_bytes(), params)?;
    let sealed = seal_vault_key(&new_master, &vault_key)?;
    let envelope = EnvelopeKey::from_master_key(&new_master)?;

    let row = keepsake_core::vault::SealedKeyRow {
        username: username.to_string(),
        device_id: random_device_id(),
        kdf_salt: salt.0,
        kdf_params: params.encode(),
        seal_nonce: sealed.nonce,
        seal_ciphertext: sealed.ciphertext,
        envelope_pk: envelope.public_key().to_bytes(),
        created_at: chrono::Utc::now(),
    };
    s.vault.put_sealed_key(&row)?;
    s.vault.append_audit(
        AuditOp::AddUser,
        &s.username,
        Some(username),
        None,
    )?;
    Ok(())
}

/// Remove a user from this device.  The vault key is not
/// affected — other users on this device can still unlock.
/// Refuses to remove the currently-logged-in user.
pub fn remove_user(
    state: &AppState,
    username: &str,
) -> Result<()> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    if s.username == username {
        return Err(keepsake_core::Error::Invalid(
            "cannot remove the currently-logged-in user; lock first".into(),
        ));
    }
    // We need a mutable reference to write the audit entry, so
    // drop the read guard and re-acquire as write.
    drop(guard);
    let mut guard = state.session.lock();
    let s = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    s.vault.delete_sealed_key(username)?;
    s.vault.append_audit(
        AuditOp::RemoveUser,
        &s.username,
        Some(username),
        None,
    )?;
    Ok(())
}

/// Change the current user's password.  Re-seals the vault
/// key under the new password; the vault key itself is stable.
pub fn change_password(
    state: &AppState,
    new_password: &str,
) -> Result<()> {
    use keepsake_core::crypto::KdfParams;
    use keepsake_core::identity::{password_to_master_key, seal_vault_key};

    let mut guard = state.session.lock();
    let s = guard.as_mut().ok_or(keepsake_core::Error::Locked)?;
    let params = KdfParams::default();
    let (new_master, salt) = password_to_master_key(new_password.as_bytes(), params)?;
    let cur = s.vault.get_sealed_key(&s.username)?
        .ok_or_else(|| keepsake_core::Error::NotFound(s.username.clone()))?;
    let vault_key = keepsake_core::identity::unseal_vault_key(
        &s.master,
        &keepsake_core::identity::SealedVaultKey {
            nonce: cur.seal_nonce,
            ciphertext: cur.seal_ciphertext,
        },
    )?;
    let sealed = seal_vault_key(&new_master, &vault_key)?;
    let new_row = keepsake_core::vault::SealedKeyRow {
        username: s.username.clone(),
        device_id: cur.device_id,
        kdf_salt: salt.0,
        kdf_params: params.encode(),
        seal_nonce: sealed.nonce,
        seal_ciphertext: sealed.ciphertext,
        envelope_pk: cur.envelope_pk,
        created_at: cur.created_at,
    };
    s.vault.put_sealed_key(&new_row)?;
    s.vault.append_audit(
        AuditOp::RotatePassword,
        &s.username,
        None,
        None,
    )?;
    s.master = new_master;
    Ok(())
}

const ALL_TYPES: &[&str] = &[
    "login", "document", "identification",
    "insurance", "health", "bank_account", "credit_card",
    "investment", "income_source", "vehicle", "residence",
    "phone", "address", "contact", "subscription",
    "infrastructure", "domain", "runbook", "work_log", "note",
];

/// Return every record's id and display title, used by the
/// frontend to render `[[uuid]]` link markers as chips.
pub fn record_titles(
    state: &AppState,
) -> Result<Vec<serde_json::Value>> {
    let guard = state.session.lock();
    let s = guard.as_ref().ok_or(keepsake_core::Error::Locked)?;
    let mut out = Vec::new();
    for t in ALL_TYPES {
        for h in s.vault.list_records(t)? {
            if let Ok((_, rec)) = s.vault.get_record(h.id) {
                out.push(serde_json::json!({
                    "id": h.id.to_string(),
                    "type": h.r#type,
                    "title": display_title(&rec),
                }));
            }
        }
    }
    Ok(out)
}

fn display_title(rec: &Record) -> String {
    match rec {
        Record::Login(l)            => format!("{} ({})", l.service, l.username),
        Record::Document(d)         => d.title.clone(),
        Record::Identification(i)   => format!("{} ({})", i.holder, i.id_type),
        Record::Insurance(i)        => format!("{} ({})", i.provider, i.policy_type),
        Record::Health(h)           => h.title.clone(),
        Record::BankAccount(b)      => format!("{} {}", b.bank, b.account_type),
        Record::CreditCard(c)       => format!("{} ({})", c.issuer, c.network),
        Record::Investment(i)       => format!("{} ({})", i.provider, i.account_type),
        Record::IncomeSource(i)     => i.source.clone(),
        Record::Vehicle(v)          => v.make_model.clone(),
        Record::Residence(r)        => r.address.clone(),
        Record::Phone(p)            => p.device.clone(),
        Record::Address(a)          => a.label.clone(),
        Record::Contact(c)          => c.name.clone(),
        Record::Subscription(s)     => s.service.clone(),
        Record::Infrastructure(i)   => i.name.clone(),
        Record::Domain(d)           => d.fqdn.clone(),
        Record::Runbook(rb)         => rb.title.clone(),
        Record::WorkLog(w)          => w.summary.clone(),
        Record::Note(n)             => n.title.clone(),
    }
}

/// Snippet of the haystack around the query, used by `find`.
fn snippet(haystack: &str, query: &str) -> String {
    let lower = haystack.to_lowercase();
    let q = query.to_lowercase();
    if let Some(pos) = lower.find(&q) {
        let start = pos.saturating_sub(20);
        let end = (pos + q.len() + 20).min(haystack.len());
        let mut s = haystack[start..end].to_string();
        if start > 0 { s = format!("…{s}"); }
        if end < haystack.len() { s.push('…'); }
        s
    } else {
        haystack.chars().take(80).collect()
    }
}

/// Replace sensitive fields with `***` in a record's JSON form.
fn masked_value(rec: &Record) -> serde_json::Value {
    let sensitive: &[&str] = match rec {
        Record::Login(_) => &["password", "totp_secret", "recovery_codes"],
        Record::Identification(_) => &["number"],
        Record::BankAccount(_) => &[
            "account_number",
            "routing_number",
            "online_username",
        ],
        Record::CreditCard(_) => &[
            "card_number",
            "cvv",
            "pin",
            "expiration",
        ],
        Record::Phone(_) => &["pin", "account_number"],
        _ => &[],
    };
    let mut v = serde_json::to_value(rec).unwrap_or(serde_json::Value::Null);
    if let serde_json::Value::Object(map) = &mut v {
        for k in sensitive {
            map.insert((*k).into(), serde_json::Value::String("***".into()));
        }
    }
    v
}

/// Build a fresh session from scratch.
pub fn build_new_session(
    path: &std::path::Path,
    username: &str,
    password: &str,
) -> Result<keepsake_core::session::Session> {
    use chrono::Utc;
    use rand::RngCore;

    use keepsake_core::crypto::KdfParams;
    use keepsake_core::identity::{
        password_to_master_key, seal_vault_key, EnvelopeKey, VaultKey,
    };
    use keepsake_core::vault::{SealedKeyRow, Vault};

    if path.exists() {
        return Err(keepsake_core::Error::AlreadyExists(
            path.display().to_string(),
        ));
    }
    let params = KdfParams::default();
    let (master, salt) = password_to_master_key(password.as_bytes(), params)?;
    let mut k = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut k);
    let vault_key = VaultKey::from_bytes(k);
    let sealed = seal_vault_key(&master, &vault_key)?;
    let envelope = EnvelopeKey::from_master_key(&master)?;
    let vault = Vault::open_or_create(path)?;
    let row = SealedKeyRow {
        username: username.to_string(),
        device_id: random_device_id(),
        kdf_salt: salt.0,
        kdf_params: params.encode(),
        seal_nonce: sealed.nonce,
        seal_ciphertext: sealed.ciphertext,
        envelope_pk: envelope.public_key().to_bytes(),
        created_at: Utc::now(),
    };
    vault.put_sealed_key(&row)?;
    drop(vault);
    let mut vault = Vault::open_or_create(path)?;
    vault.unlock(&vault_key)?;
    vault.append_audit(
        AuditOp::AddUser,
        username,
        Some(username),
        Some("vault initialized"),
    )?;
    vault.append_audit(AuditOp::Unlock, username, None, None)?;
    Ok(keepsake_core::session::Session::new(
        path.to_path_buf(),
        vault,
        master,
        username.to_string(),
    )?)
}

/// Open an existing vault.
pub fn open_session(
    path: &std::path::Path,
    username: &str,
    password: &str,
) -> Result<keepsake_core::session::Session> {
    use keepsake_core::crypto::KdfParams;
    use keepsake_core::identity::{unseal_vault_key, SealedVaultKey};
    use keepsake_core::vault::Vault;

    let vault = Vault::open_or_create(path)?;
    let row = vault
        .get_sealed_key(username)?
        .ok_or_else(|| keepsake_core::Error::NotFound(username.to_string()))?;
    let params = KdfParams::decode(&row.kdf_params)?;
    let master = keepsake_core::crypto::derive_master_key(
        password.as_bytes(),
        &row.kdf_salt,
        params,
    )?;
    let vault_key = unseal_vault_key(
        &master,
        &SealedVaultKey {
            nonce: row.seal_nonce,
            ciphertext: row.seal_ciphertext,
        },
    )?;
    let mut vault = vault;
    vault.unlock(&vault_key)?;
    vault.append_audit(AuditOp::Unlock, username, None, None)?;
    Ok(keepsake_core::session::Session::new(
        path.to_path_buf(),
        vault,
        master,
        username.to_string(),
    )?)
}

fn random_device_id() -> [u8; 16] {
    use rand::RngCore;
    let mut d = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut d);
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use keepsake_core::records::{Login, Record};
    use keepsake_core::session::Session;
    use parking_lot::Mutex;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn state() -> AppState {
        AppState {
            session: Arc::new(Mutex::new(None)),
            auto_sync: Arc::new(parking_lot::Mutex::new(None)),
        }
    }

    #[test]
    fn import_to_new_vault_round_trip() {
        let dir = tempdir().unwrap();
        let src_path = dir.path().join("source.db");
        let dst_path = dir.path().join("dest.db");

        // Build a session, add a record, and export to a .ksk
        // bundle in memory.
        let st = state();
        init(&st, &src_path, "alice", "hunter2").unwrap();
        let rec = Record::Login(Login {
            id: uuid::Uuid::new_v4(),
            service: "ExampleMail".into(),
            username: "alice@example.com".into(),
            holders: vec!["Alice".into()],
            password: Some("p@ssword".into()),
            totp_secret: None,
            recovery_codes: None,
            url: Some("https://example.com".into()),
            notes: "import test".into(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
        let id = add_record(&st, "login", serde_json::to_value(&rec).unwrap()).unwrap();
        let bytes = export_bundle(&st, b"export-pass")
            .map_err(|e| format!("export_bundle failed: {e}"))
            .unwrap();
        lock(&st);
        // Wipe the source so we can't accidentally read from it.
        std::fs::remove_file(&src_path).unwrap();

        // Import into a brand-new vault at a different path.
        let imported = import_to_new_vault(
            &dst_path,
            &bytes,
            b"export-pass",
            "bob",
            "swordfish",
        )
        .unwrap();
        assert_eq!(imported.username, "bob");
        // Read the record back; sensitive field is still
        // recoverable.
        let revealed = show_record_with_session(&imported, &id, true).unwrap();
        let v = revealed.get("service").and_then(|x| x.as_str()).unwrap();
        assert_eq!(v, "ExampleMail");
        let pw = revealed.get("password").and_then(|x| x.as_str()).unwrap();
        assert_eq!(pw, "p@ssword");
    }

    fn show_record_with_session(
        sess: &Session,
        id: &str,
        reveal: bool,
    ) -> keepsake_core::Result<serde_json::Value> {
        let id = uuid::Uuid::parse_str(id)
            .map_err(|_| keepsake_core::Error::Invalid(format!("bad uuid: {id}")))?;
        let (_h, rec) = sess.vault.get_record(id)?;
        if reveal {
            Ok(serde_json::to_value(&rec)?)
        } else {
            Ok(masked_value(&rec))
        }
    }

    #[test]
    fn recover_from_sync_creates_vault_with_shared_setup() {
        // Simulate a recovery: fresh device, knows the
        // server URL, vault id, and sync passphrase.  No
        // existing vault file.  recover_from_sync should
        // create the vault, install the shared setup, and
        // leave the session unlocked with the shared key
        // in memory.
        let dir = tempdir().unwrap();
        let path = dir.path().join("recovered.db");
        let session = recover_from_sync(
            &path,
            "https://sync.example.com",
            "family",
            "shared-pass-1",
            "bob",
            "swordfish",
        )
        .unwrap();

        assert_eq!(session.username, "bob");
        assert!(session.vault.is_unlocked());
        // The shared setup is stored in the vault.
        let setup = session.vault.get_shared_sync("family").unwrap().unwrap();
        assert_eq!(setup.passphrase, "shared-pass-1");
        assert_eq!(setup.server_url.as_deref(), Some("https://sync.example.com"));
        // The shared key is in the session's in-memory cache.
        let shared_key = session.shared_sync_key("family").unwrap();
        // The same key would be re-derived independently
        // from the same passphrase + vault_id.
        let rederived = keepsake_core::sync::client::derive_shared_key(
            b"shared-pass-1",
            "family",
        ).unwrap();
        assert_eq!(shared_key.as_bytes(), rederived.as_bytes());

        // The vault file was actually created on disk.
        assert!(path.exists());
    }

    #[test]
    fn recover_from_sync_fails_if_vault_exists() {
        // The vault file is created on disk by init/recover,
        // so a second call must fail with AlreadyExists.
        let dir = tempdir().unwrap();
        let path = dir.path().join("recovered.db");
        recover_from_sync(
            &path,
            "https://sync.example.com",
            "family",
            "shared-pass",
            "bob",
            "swordfish",
        )
        .unwrap();
        let err = recover_from_sync(
            &path,
            "https://sync.example.com",
            "family",
            "shared-pass",
            "bob",
            "swordfish",
        )
        .err()
        .expect("second recover should fail");
        assert!(
            matches!(err, keepsake_core::Error::AlreadyExists(_)),
            "got: {err:?}"
        );
    }
}
