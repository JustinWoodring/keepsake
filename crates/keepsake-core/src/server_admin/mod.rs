//! Server admin key.  Used by the standalone sync server for
//! operational tasks (TLS cert renewal, log rotation, disk
//! monitoring).  Has no relationship to any vault key and
//! cannot be used to decrypt vault contents.

use rand::RngCore;

use crate::error::Result;
use crate::crypto::zeroize::KEY_LEN;

/// A 32-byte server admin key.
#[derive(Clone)]
pub struct ServerAdminKey([u8; KEY_LEN]);

impl ServerAdminKey {
    /// Generate a fresh random key.
    pub fn generate() -> Self {
        let mut k = [0u8; KEY_LEN];
        rand::thread_rng().fill_bytes(&mut k);
        Self(k)
    }

    /// Wrap a known key.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl std::fmt::Debug for ServerAdminKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerAdminKey").finish_non_exhaustive()
    }
}

/// Load a key from a file (raw 32 bytes).  Used at server startup
/// when the key is provisioned via systemd `LoadCredential=` or
/// an env var.
pub fn load_from_file(path: impl AsRef<std::path::Path>) -> Result<ServerAdminKey> {
    let bytes = std::fs::read(path.as_ref())?;
    if bytes.len() != KEY_LEN {
        return Err(crate::Error::Invalid(format!(
            "admin key must be {} bytes, got {}",
            KEY_LEN,
            bytes.len()
        )));
    }
    let mut out = [0u8; KEY_LEN];
    out.copy_from_slice(&bytes);
    Ok(ServerAdminKey(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn round_trip_through_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("admin.key");
        let k = ServerAdminKey::generate();
        std::fs::write(&path, k.as_bytes()).unwrap();
        let k2 = load_from_file(&path).unwrap();
        assert_eq!(k.as_bytes(), k2.as_bytes());
    }
}
