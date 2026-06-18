//! Ed25519 signing and verification (envelope authentication).

use ed25519_dalek::{Signer, Verifier};
use rand::rngs::OsRng;

use crate::error::{Error, Result};

/// Ed25519 public key length (bytes).
pub const ED25519_PK_LEN: usize = 32;
/// Ed25519 signature length (bytes).
pub const ED25519_SIG_LEN: usize = 64;

/// Ed25519 signing key (private).  Wiped on drop.
pub struct SigningKey(ed25519_dalek::SigningKey);

/// Ed25519 verifying key (public).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct VerifyingKey(ed25519_dalek::VerifyingKey);

/// Ed25519 signature.
#[derive(Clone, Copy)]
pub struct Signature(ed25519_dalek::Signature);

impl SigningKey {
    /// Generate a fresh random signing key.
    pub fn generate() -> Self {
        Self(ed25519_dalek::SigningKey::generate(&mut OsRng))
    }

    /// Reconstruct a signing key from a 32-byte seed.
    pub fn from_bytes(seed: &[u8; 32]) -> Result<Self> {
        Ok(Self(ed25519_dalek::SigningKey::from_bytes(seed)))
    }

    /// Borrow the public verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey(self.0.verifying_key())
    }

    /// Sign a message.
    pub fn sign(&self, msg: &[u8]) -> Signature {
        Signature(self.0.sign(msg))
    }

    /// Borrow the raw 32-byte seed.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0.to_bytes()
    }
}

impl Drop for SigningKey {
    fn drop(&mut self) {
        // ed25519-dalek 2.x zeroes its internal state on drop.
    }
}

impl VerifyingKey {
    /// Build from a 32-byte public key.
    pub fn from_bytes(bytes: &[u8; ED25519_PK_LEN]) -> Result<Self> {
        ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map(Self)
            .map_err(|e| Error::Crypto(format!("verifying key: {e}")))
    }

    /// Borrow the raw 32-byte public key.
    pub fn to_bytes(&self) -> [u8; ED25519_PK_LEN] {
        self.0.to_bytes()
    }
}

impl std::fmt::Debug for VerifyingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifyingKey").finish_non_exhaustive()
    }
}

impl Signature {
    /// Build from a 64-byte signature.
    pub fn from_bytes(bytes: &[u8; ED25519_SIG_LEN]) -> Result<Self> {
        Ok(Self(ed25519_dalek::Signature::from_bytes(bytes)))
    }

    /// Borrow the raw 64-byte signature.
    pub fn to_bytes(&self) -> [u8; ED25519_SIG_LEN] {
        self.0.to_bytes()
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signature").finish_non_exhaustive()
    }
}

/// Verify a signature against a message and public key.
pub fn verify_signature(key: &VerifyingKey, msg: &[u8], sig: &Signature) -> Result<()> {
    key.0
        .verify(msg, &sig.0)
        .map_err(|_| Error::Crypto("signature verification failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_round_trip() {
        let sk = SigningKey::generate();
        let pk = sk.verifying_key();
        let msg = b"server challenge || body hash";
        let sig = sk.sign(msg);
        verify_signature(&pk, msg, &sig).unwrap();
        assert!(verify_signature(&pk, b"different", &sig).is_err());
    }
}
