//! Sealed vault key — the on-disk form of the symmetric vault key,
//! encrypted with the user's master key.

use crate::crypto::{aead, AeadKey, Nonce};
use crate::error::Result;

use super::MasterKey;

/// A 32-byte random vault key.
pub struct VaultKey(AeadKey);

impl VaultKey {
    /// Wrap a 32-byte key.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(AeadKey::from_bytes(bytes))
    }

    /// Borrow the raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl std::fmt::Debug for VaultKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VaultKey").finish_non_exhaustive()
    }
}

/// On-disk sealed form: nonce || ciphertext-and-tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SealedVaultKey {
    /// 24-byte nonce.
    pub nonce: [u8; 24],
    /// 32-byte plaintext + 16-byte tag.
    pub ciphertext: Vec<u8>,
}

/// Seal a vault key under a master key.  The plaintext is the 32-byte
/// vault key; the AAD is the string `"vault_key/v1"` so a future
/// format version can be unambiguously distinguished.
pub fn seal_vault_key(master: &MasterKey, vault: &VaultKey) -> Result<SealedVaultKey> {
    let key = AeadKey::from_bytes({
        let mut b = [0u8; 32];
        b.copy_from_slice(master.as_bytes());
        b
    });
    let nonce = Nonce::random();
    let ct = aead::encrypt(&key, &nonce, vault.as_bytes(), b"vault_key/v1")?;
    Ok(SealedVaultKey {
        nonce: nonce.0,
        ciphertext: ct,
    })
}

/// Unseal a vault key.  Returns an error if the AAD doesn't match or
/// the ciphertext has been tampered with.
pub fn unseal_vault_key(master: &MasterKey, sealed: &SealedVaultKey) -> Result<VaultKey> {
    let key = AeadKey::from_bytes({
        let mut b = [0u8; 32];
        b.copy_from_slice(master.as_bytes());
        b
    });
    let nonce = Nonce::from_bytes(sealed.nonce);
    let pt = aead::decrypt(&key, &nonce, &sealed.ciphertext, b"vault_key/v1")?;
    if pt.len() != 32 {
        return Err(crate::Error::Crypto(format!(
            "unsealed vault key has wrong length: {}",
            pt.len()
        )));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&pt);
    Ok(VaultKey::from_bytes(out))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{derive_master_key, KdfParams};

    fn mk_master() -> MasterKey {
        derive_master_key(
            b"password",
            b"unit-test-salt-xx",
            KdfParams { m_kib: 8 * 1024, t: 1, p: 1 },
        )
        .unwrap()
    }

    #[test]
    fn seal_unseal_round_trip() {
        let mk = mk_master();
        let vk = VaultKey::from_bytes([7u8; 32]);
        let sealed = seal_vault_key(&mk, &vk).unwrap();
        let vk2 = unseal_vault_key(&mk, &sealed).unwrap();
        assert_eq!(vk.as_bytes(), vk2.as_bytes());
    }

    #[test]
    fn unseal_wrong_password_fails() {
        let mk = mk_master();
        let vk = VaultKey::from_bytes([7u8; 32]);
        let sealed = seal_vault_key(&mk, &vk).unwrap();
        let other = derive_master_key(
            b"different",
            b"unit-test-salt-xx",
            KdfParams { m_kib: 8 * 1024, t: 1, p: 1 },
        )
        .unwrap();
        assert!(unseal_vault_key(&other, &sealed).is_err());
    }
}
