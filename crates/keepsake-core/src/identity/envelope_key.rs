//! Envelope keypair — used for server authentication.  The signing
//! key is derived from the user's master key via HKDF; this keeps
//! "the secret" to a single thing (the password) while still giving
//! the server a stable per-user identity.

use crate::crypto::{derive_subkey, sign, SigningKey, VerifyingKey, ED25519_PK_LEN};
use crate::error::Result;

use super::MasterKey;

/// 32-byte Ed25519 seed derived from the master key.
const ENVELOPE_DOMAIN: &[u8] = b"keepsake/envelope/v1";

/// 32-byte bearer secret derived from the master key (server auth).
pub const BEARER_DOMAIN: &[u8] = b"keepsake/bearer/v1";

/// Per-user envelope (server-auth) keypair.
pub struct EnvelopeKey {
    signing: SigningKey,
    verifying: VerifyingKey,
}

impl EnvelopeKey {
    /// Derive an envelope keypair from a master key.
    pub fn from_master_key(master: &MasterKey) -> Result<Self> {
        let seed_v = derive_subkey(master.as_bytes(), &[], ENVELOPE_DOMAIN, 32)?;
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&seed_v);
        let signing = SigningKey::from_bytes(&seed)?;
        let verifying = signing.verifying_key();
        Ok(Self { signing, verifying })
    }

    /// Borrow the public verifying key.
    pub fn public_key(&self) -> EnvelopePublicKey {
        EnvelopePublicKey(self.verifying)
    }

    /// Borrow the inner signing key (used by the transport layer to
    /// sign requests).
    pub(crate) fn signing_key(&self) -> &SigningKey {
        &self.signing
    }
}

/// A 32-byte Ed25519 public key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvelopePublicKey(VerifyingKey);

impl EnvelopePublicKey {
    /// Wrap raw bytes.
    pub fn from_bytes(bytes: &[u8; ED25519_PK_LEN]) -> Result<Self> {
        Ok(Self(VerifyingKey::from_bytes(bytes)?))
    }

    /// Borrow the raw bytes.
    pub fn to_bytes(&self) -> [u8; ED25519_PK_LEN] {
        self.0.to_bytes()
    }
}

/// Derive a 32-byte bearer secret from the master key.
pub fn derive_bearer(master: &MasterKey) -> Result<[u8; 32]> {
    let v = derive_subkey(master.as_bytes(), &[], BEARER_DOMAIN, 32)?;
    let mut out = [0u8; 32];
    out.copy_from_slice(&v);
    Ok(out)
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
    fn envelope_key_derives_deterministically() {
        let mk = mk_master();
        let a = EnvelopeKey::from_master_key(&mk).unwrap();
        let b = EnvelopeKey::from_master_key(&mk).unwrap();
        assert_eq!(
            a.public_key().to_bytes(),
            b.public_key().to_bytes()
        );
    }

    #[test]
    fn envelope_key_changes_with_password() {
        let mk1 = mk_master();
        let mk2 = derive_master_key(
            b"different",
            b"unit-test-salt-xx",
            KdfParams { m_kib: 8 * 1024, t: 1, p: 1 },
        )
        .unwrap();
        let a = EnvelopeKey::from_master_key(&mk1).unwrap();
        let b = EnvelopeKey::from_master_key(&mk2).unwrap();
        assert_ne!(a.public_key().to_bytes(), b.public_key().to_bytes());
    }

    #[test]
    fn bearer_changes_with_password() {
        let mk1 = mk_master();
        let mk2 = derive_master_key(
            b"different",
            b"unit-test-salt-xx",
            KdfParams { m_kib: 8 * 1024, t: 1, p: 1 },
        )
        .unwrap();
        assert_ne!(derive_bearer(&mk1).unwrap(), derive_bearer(&mk2).unwrap());
    }
}
