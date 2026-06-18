//! In-memory session for the CLI.
//!
//! Lives in the core crate so the CLI and the future Tauri app
//! share the same shape.  The session is intentionally not
//! `Sync` — the CLI holds it in a `Mutex`/`RwLock` in its own
//! crate.

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
}
