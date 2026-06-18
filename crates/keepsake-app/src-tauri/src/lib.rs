// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Tauri shell for Keepsake.  All business logic lives in
//! `keepsake-core`; this crate exposes it to the Solid
//! frontend via Tauri commands.

use std::sync::Arc;

use parking_lot::Mutex;
use tauri::State;

mod session;

/// Per-app state.  The session is held behind a Mutex; the
/// vault inside is locked when this is `None`.
pub struct AppState {
    pub session: Arc<Mutex<Option<keepsake_core::session::Session>>>,
}

fn default_vault_path() -> std::path::PathBuf {
    keepsake_core::config::default_vault_path()
}

/// Initialize a new vault on this device.  If `path` is empty,
/// uses the default per-OS location.
#[tauri::command]
async fn init(
    state: State<'_, AppState>,
    path: Option<String>,
    username: String,
    password: String,
) -> Result<(), String> {
    let p = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_vault_path);
    session::init(&state, &p, &username, &password).map_err(|e| e.to_string())
}

/// Unlock an existing vault.
#[tauri::command]
async fn unlock(
    state: State<'_, AppState>,
    path: Option<String>,
    username: String,
    password: String,
) -> Result<(), String> {
    let p = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_vault_path);
    session::unlock(&state, &p, &username, &password).map_err(|e| e.to_string())
}

/// Lock the vault, zeroizing the in-memory key.
#[tauri::command]
async fn lock(state: State<'_, AppState>) -> Result<(), String> {
    session::lock(&state);
    Ok(())
}

/// Returns true if the vault is currently unlocked, and the
/// username.
#[tauri::command]
async fn status(
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let g = state.session.lock();
    Ok(serde_json::json!({
        "unlocked": g.is_some(),
        "username": g.as_ref().map(|s| s.username.clone()),
    }))
}

/// List the usernames that have a sealed row on this device.
/// Returns an empty list if the vault file doesn't exist.
#[tauri::command]
async fn list_users(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<Vec<String>, String> {
    let p = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_vault_path);
    session::list_users(&state, &p).map_err(|e| e.to_string())
}

/// Add a record.  Returns the new record's id as a string.
#[tauri::command]
async fn add_record(
    state: State<'_, AppState>,
    r#type: String,
    fields: serde_json::Value,
) -> Result<String, String> {
    session::add_record(&state, &r#type, fields).map_err(|e| e.to_string())
}

/// Update a record by id.  `fields` is the full new record body.
#[tauri::command]
async fn update_record(
    state: State<'_, AppState>,
    id: String,
    fields: serde_json::Value,
) -> Result<(), String> {
    session::update_record(&state, &id, fields).map_err(|e| e.to_string())
}

/// Delete a record by id.
#[tauri::command]
async fn delete_record(
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    session::delete_record(&state, &id).map_err(|e| e.to_string())
}

/// List records of a given type.  Returns an array of
/// `{id, type, updated_by, updated_at}` objects.
#[tauri::command]
async fn list_records(
    state: State<'_, AppState>,
    r#type: String,
) -> Result<Vec<serde_json::Value>, String> {
    session::list_records(&state, &r#type).map_err(|e| e.to_string())
}

/// Fetch a record by id.  Sensitive fields are masked unless
/// `reveal` is true.
#[tauri::command]
async fn show_record(
    state: State<'_, AppState>,
    id: String,
    reveal: bool,
) -> Result<serde_json::Value, String> {
    session::show_record(&state, &id, reveal).map_err(|e| e.to_string())
}

/// Free-text search.
#[tauri::command]
async fn find(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<serde_json::Value>, String> {
    session::find(&state, &query).map_err(|e| e.to_string())
}

/// Read or verify the audit chain.
#[tauri::command]
async fn audit(
    state: State<'_, AppState>,
    verify: bool,
) -> Result<serde_json::Value, String> {
    session::audit(&state, verify).map_err(|e| e.to_string())
}

/// Configure the sync server base URL.
#[tauri::command]
async fn configure_sync(
    state: State<'_, AppState>,
    base_url: String,
) -> Result<(), String> {
    session::configure_sync(&state, &base_url).map_err(|e| e.to_string())
}

/// Return every record's id and display title, used by the
/// frontend to render `[[uuid]]` link markers as chips.
#[tauri::command]
async fn record_titles(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    session::record_titles(&state).map_err(|e| e.to_string())
}

/// Rebuild the audit chain in place.  Drops entries that don't
/// hash correctly under the current `entry_hash` and re-chains
/// the survivors.  Returns the number of entries dropped.
#[tauri::command]
async fn rewrite_audit_chain(
    state: State<'_, AppState>,
) -> Result<usize, String> {
    session::rewrite_audit_chain(&state).map_err(|e| e.to_string())
}

/// Push every local record to the sync server.  Returns the
/// number of records pushed.
#[tauri::command]
async fn sync_push(
    state: State<'_, AppState>,
    server_url: String,
    vault_id: String,
) -> Result<usize, String> {
    session::sync_push(&state, server_url, vault_id)
        .await
        .map_err(|e| e.to_string())
}

/// Pull changes from the sync server and apply them locally.
/// Returns the number of changes applied.
#[tauri::command]
async fn sync_pull(
    state: State<'_, AppState>,
    server_url: String,
    vault_id: String,
) -> Result<usize, String> {
    session::sync_pull(&state, server_url, vault_id)
        .await
        .map_err(|e| e.to_string())
}

/// Set up (or rotate) the shared sync setup for `vault_id`.
/// Seals the passphrase inside the vault.
#[tauri::command]
async fn setup_shared_sync(
    state: State<'_, AppState>,
    vault_id: String,
    passphrase: String,
) -> Result<(), String> {
    session::setup_shared_sync(&state, vault_id, passphrase).map_err(|e| e.to_string())
}

/// Reveal the shared sync setup for `vault_id`.  Returns
/// `(vault_id, passphrase)`.  The user can copy this to
/// another device to configure sync there.
#[tauri::command]
async fn reveal_shared_sync(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<(String, String), String> {
    session::reveal_shared_sync(&state, vault_id).map_err(|e| e.to_string())
}

/// Delete the shared sync setup for `vault_id`.
#[tauri::command]
async fn delete_shared_sync(
    state: State<'_, AppState>,
    vault_id: String,
) -> Result<(), String> {
    session::delete_shared_sync(&state, vault_id).map_err(|e| e.to_string())
}

/// List the vault ids that have a shared sync setup on this
/// device.  Used by the Sync page to populate the dropdown.
#[tauri::command]
async fn list_shared_syncs(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let guard = state.session.lock();
    let session = guard.as_ref().ok_or_else(|| {
        keepsake_core::Error::Locked.to_string()
    })?;
    session.vault.list_shared_syncs().map_err(|e| e.to_string())
}

/// Add a new user to this device's vault.
#[tauri::command]
async fn add_user(
    state: State<'_, AppState>,
    username: String,
    password: String,
) -> Result<(), String> {
    session::add_user(&state, &username, &password).map_err(|e| e.to_string())
}

/// Remove a user from this device.
#[tauri::command]
async fn remove_user(
    state: State<'_, AppState>,
    username: String,
) -> Result<(), String> {
    session::remove_user(&state, &username).map_err(|e| e.to_string())
}

/// Change the current user's password.
#[tauri::command]
async fn change_password(
    state: State<'_, AppState>,
    new_password: String,
) -> Result<(), String> {
    session::change_password(&state, &new_password).map_err(|e| e.to_string())
}

/// Export the vault to a `.ksk` bundle.  Returns the raw bytes
/// of the bundle; the frontend writes them to disk.
#[tauri::command]
async fn export_bundle(
    state: State<'_, AppState>,
    passphrase: String,
) -> Result<Vec<u8>, String> {
    session::export_bundle(&state, passphrase.as_bytes()).map_err(|e| e.to_string())
}

/// Import a `.ksk` bundle.
#[tauri::command]
async fn import_bundle(
    state: State<'_, AppState>,
    bytes: Vec<u8>,
    passphrase: String,
) -> Result<(), String> {
    session::import_bundle(&state, &bytes, passphrase.as_bytes()).map_err(|e| e.to_string())
}

/// Import a `.ksk` bundle into a brand-new vault.  Used by the
/// unlock screen when no vault file exists yet.  After the
/// import succeeds, the session is set so the user lands
/// signed in.
#[tauri::command]
async fn import_to_new_vault(
    state: State<'_, AppState>,
    path: Option<String>,
    bytes: Vec<u8>,
    passphrase: String,
    username: String,
    password: String,
) -> Result<(), String> {
    let p = path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_vault_path);
    let mut guard = state.session.lock();
    if guard.is_some() {
        return Err("vault is already unlocked".into());
    }
    let session = session::import_to_new_vault(
        &p,
        &bytes,
        passphrase.as_bytes(),
        &username,
        &password,
    )
    .map_err(|e| e.to_string())?;
    *guard = Some(session);
    Ok(())
}

/// Returns the default vault path for the current OS.
#[tauri::command]
async fn default_path() -> Result<String, String> {
    Ok(default_vault_path().display().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            session: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            init,
            unlock,
            lock,
            status,
            list_users,
            add_user,
            remove_user,
            change_password,
            export_bundle,
            import_bundle,
            import_to_new_vault,
            add_record,
            update_record,
            delete_record,
            list_records,
            show_record,
            find,
            audit,
            configure_sync,
            record_titles,
            rewrite_audit_chain,
            sync_push,
            sync_pull,
            setup_shared_sync,
            reveal_shared_sync,
            delete_shared_sync,
            list_shared_syncs,
            default_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
