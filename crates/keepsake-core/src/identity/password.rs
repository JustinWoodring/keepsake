//! Password → master key.

use rand::RngCore;

use crate::crypto::{derive_master_key, KdfParams, MasterKey, KEY_LEN};
use crate::error::Result;

/// A 16-byte Argon2id salt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Salt(pub [u8; 16]);

impl Salt {
    /// Generate a fresh random salt.
    pub fn random() -> Self {
        let mut s = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut s);
        Self(s)
    }

    /// Construct from raw bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
}

/// Derive a master key from `password` using a fresh random salt
/// and the provided KDF parameters.  Returns both the master key and
/// the salt (which must be persisted so the password can be re-derived
/// later).
pub fn password_to_master_key(
    password: &[u8],
    params: KdfParams,
) -> Result<(MasterKey, Salt)> {
    let salt = Salt::random();
    let mk = derive_master_key(password, salt.as_bytes(), params)?;
    Ok((mk, salt))
}

impl Salt {
    /// Borrow the raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_key_round_trip() {
        let params = KdfParams {
            m_kib: 8 * 1024,
            t: 1,
            p: 1,
        };
        let (mk1, salt) = password_to_master_key(b"hunter2", params).unwrap();
        let mk2 = derive_master_key(b"hunter2", salt.as_bytes(), params).unwrap();
        assert_eq!(mk1.as_bytes(), mk2.as_bytes());
        // KEY_LEN is always 32 for this profile.
        assert_eq!(mk1.as_bytes().len(), KEY_LEN);
    }
}
