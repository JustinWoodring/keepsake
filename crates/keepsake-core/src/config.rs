//! Configuration constants and small structs used across the library.

use std::path::PathBuf;

/// Recommended Argon2id parameters.  Roughly 200ms on a modern laptop.
pub const ARGON2_M_KIB: u32 = 64 * 1024;
pub const ARGON2_T:     u32 = 3;
pub const ARGON2_P:     u32 = 4;

/// Size of the master key, vault key, and AEAD keys (bytes).
pub const KEY_LEN: usize = 32;

/// Size of an XChaCha20-Poly1305 nonce (bytes).
pub const NONCE_LEN: usize = 24;

/// Size of a Poly1305 authentication tag (bytes).
pub const TAG_LEN: usize = 16;

/// Ed25519 public key size.
pub const ED25519_PK_LEN: usize = 32;
/// Ed25519 signature size.
pub const ED25519_SIG_LEN: usize = 64;

/// Maximum size of an inline attachment (bytes).  256 KiB.
pub const ATTACHMENT_INLINE_MAX: usize = 256 * 1024;

/// Current vault schema version.  Bump on breaking schema changes.
pub const VAULT_SCHEMA_VERSION: u32 = 1;

/// Default location of the vault file for the current OS.
pub fn default_vault_path() -> PathBuf {
    let base = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("keepsake").join("vault.db")
}
