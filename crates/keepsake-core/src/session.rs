//! In-memory session for the CLI.
//!
//! Lives in the core crate so the CLI and the future Tauri app
//! share the same shape.  The session is intentionally not
//! `Sync` — the CLI holds it in a `Mutex`/`RwLock` in its own
//! crate.

use std::collections::HashMap;

use crate::crypto::AeadKey;
use crate::identity::MasterKey;
use crate::vault::Vault;

/// An unlocked vault session.
pub struct Session {
    /// Path to the vault file.
    pub path: std::path::PathBuf,
    /// The vault itself (owns the SQLite connection).
    pub vault: Vault,
    /// The current user's master key.  Zeroized on drop.
    pub master: MasterKey,
    /// The current username.
    pub username: String,
    /// Derived `shared_sync_key` per `vault_id`, populated
    /// from the vault's `shared_sync_keys` table at unlock
    /// time.  Cleared on lock.  Keys are zeroized on drop.
    pub shared_sync_keys: HashMap<String, AeadKey>,
}

impl Session {
    /// Construct a new session.  Populates the in-memory
    /// `shared_sync_keys` cache from the vault.  The caller
    /// must have already called `vault.unlock(&vault_key)`.
    pub fn new(
        path: std::path::PathBuf,
        vault: Vault,
        master: MasterKey,
        username: String,
    ) -> crate::error::Result<Self> {
        let mut shared_sync_keys = HashMap::new();
        for vid in vault.list_shared_syncs()? {
            if let Some(k) = vault.get_shared_sync_key(&vid)? {
                shared_sync_keys.insert(vid, k);
            }
        }
        Ok(Self { path, vault, master, username, shared_sync_keys })
    }

    /// Look up a shared sync key by `vault_id`.
    pub fn shared_sync_key(&self, vault_id: &str) -> Option<&AeadKey> {
        self.shared_sync_keys.get(vault_id)
    }

    /// Insert or replace a shared sync key (used by the
    /// setup/rotate flows after the vault has been mutated).
    pub fn refresh_shared_sync_keys(&mut self) -> crate::error::Result<()> {
        self.shared_sync_keys.clear();
        for vid in self.vault.list_shared_syncs()? {
            if let Some(k) = self.vault.get_shared_sync_key(&vid)? {
                self.shared_sync_keys.insert(vid, k);
            }
        }
        Ok(())
    }

    /// Lock the session: clear all in-memory keys.  The
    /// `Zeroizing` wrappers on `AeadKey` and `MasterKey` zero
    /// the bytes on drop; this method additionally empties
    /// the `shared_sync_keys` map and locks the vault.
    pub fn lock(&mut self) {
        self.vault.lock();
        self.shared_sync_keys.clear();
    }
}
