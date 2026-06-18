//! Argon2id password-based key derivation.

use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

use super::KEY_LEN;

/// Tunable Argon2id parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory cost in kibibytes.
    pub m_kib: u32,
    /// Time cost (iterations).
    pub t: u32,
    /// Parallelism (lanes).
    pub p: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        Self {
            m_kib: 64 * 1024,
            t: 3,
            p: 4,
        }
    }
}

impl KdfParams {
    /// Encode the parameters for on-disk storage.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12);
        out.extend_from_slice(&self.m_kib.to_le_bytes());
        out.extend_from_slice(&self.t.to_le_bytes());
        out.extend_from_slice(&self.p.to_le_bytes());
        out
    }

    /// Decode parameters from on-disk form.  Returns an error if the
    /// encoded length is wrong.
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 12 {
            return Err(Error::Crypto(format!(
                "kdf params: expected 12 bytes, got {}",
                bytes.len()
            )));
        }
        let mut m = [0u8; 4];
        let mut t = [0u8; 4];
        let mut p = [0u8; 4];
        m.copy_from_slice(&bytes[0..4]);
        t.copy_from_slice(&bytes[4..8]);
        p.copy_from_slice(&bytes[8..12]);
        Ok(Self {
            m_kib: u32::from_le_bytes(m),
            t: u32::from_le_bytes(t),
            p: u32::from_le_bytes(p),
        })
    }
}

/// A 32-byte master key derived from a password.  Wrapped in a
/// `Zeroizing<[u8; 32]>` so it is wiped on drop.
#[derive(Clone)]
pub struct MasterKey(pub zeroize::Zeroizing<[u8; KEY_LEN]>);

impl MasterKey {
    /// Borrow the underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    /// Construct from raw bytes, wrapping in a zeroizing guard.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(zeroize::Zeroizing::new(bytes))
    }
}

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MasterKey").finish_non_exhaustive()
    }
}

/// Derive a 32-byte master key from a password and salt using Argon2id.
pub fn derive_master_key(
    password: &[u8],
    salt: &[u8],
    params: KdfParams,
) -> Result<MasterKey> {
    let argon = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(params.m_kib, params.t, params.p, Some(KEY_LEN))
            .map_err(|e| Error::Crypto(format!("argon2 params: {e}")))?,
    );
    let mut out = zeroize::Zeroizing::new([0u8; KEY_LEN]);
    argon
        .hash_password_into(password, salt, out.as_mut())
        .map_err(|e| Error::Crypto(format!("argon2: {e}")))?;
    Ok(MasterKey(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kdf_params_round_trip() {
        let p = KdfParams::default();
        let bytes = p.encode();
        assert_eq!(bytes.len(), 12);
        let q = KdfParams::decode(&bytes).unwrap();
        assert_eq!(p, q);
    }

    #[test]
    fn derive_master_key_is_deterministic() {
        let p = KdfParams {
            m_kib: 8 * 1024,
            t: 1,
            p: 1,
        };
        let salt = b"unit-test-salt-1234";
        let a = derive_master_key(b"correct horse battery staple", salt, p).unwrap();
        let b = derive_master_key(b"correct horse battery staple", salt, p).unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn derive_master_key_changes_with_password() {
        let p = KdfParams {
            m_kib: 8 * 1024,
            t: 1,
            p: 1,
        };
        let salt = b"unit-test-salt-1234";
        let a = derive_master_key(b"password1", salt, p).unwrap();
        let b = derive_master_key(b"password2", salt, p).unwrap();
        assert_ne!(a.as_bytes(), b.as_bytes());
    }
}
