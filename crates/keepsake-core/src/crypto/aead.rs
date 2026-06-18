//! XChaCha20-Poly1305 authenticated encryption.

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;

use crate::error::{Error, Result};

use super::zeroize::Zeroizing;
use super::KEY_LEN;

/// XChaCha20-Poly1305 authentication tag length in bytes.
pub const TAG_LEN: usize = 16;

/// A 24-byte XChaCha20-Poly1305 nonce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Nonce(pub [u8; 24]);

impl Nonce {
    /// Generate a fresh random nonce.
    pub fn random() -> Self {
        let mut n = [0u8; 24];
        rand::thread_rng().fill_bytes(&mut n);
        Self(n)
    }

    /// Build a nonce from raw bytes.
    pub fn from_bytes(bytes: [u8; 24]) -> Self {
        Self(bytes)
    }

    /// Borrow the bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A 32-byte symmetric key.
pub struct AeadKey(pub Zeroizing<[u8; KEY_LEN]>);

impl AeadKey {
    /// Wrap raw bytes in a zeroizing guard.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Borrow the bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
}

impl Clone for AeadKey {
    fn clone(&self) -> Self {
        Self(Zeroizing::new(*self.0.clone()))
    }
}

impl std::fmt::Debug for AeadKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AeadKey").finish_non_exhaustive()
    }
}

/// Generate a fresh random 32-byte AEAD key.
pub fn random_key() -> AeadKey {
    let mut k = [0u8; KEY_LEN];
    rand::thread_rng().fill_bytes(&mut k);
    AeadKey::from_bytes(k)
}

/// Encrypt `plaintext` with the given key and nonce, authenticating
/// `aad` as associated data.  Returns ciphertext || tag.
pub fn encrypt(
    key: &AeadKey,
    nonce: &Nonce,
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
    cipher
        .encrypt(
            XNonce::from_slice(nonce.as_bytes()),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|e| Error::Crypto(format!("encrypt: {e}")))
}

/// Decrypt `ciphertext` (which includes the trailing tag) with the
/// given key and nonce, verifying `aad` as associated data.
pub fn decrypt(
    key: &AeadKey,
    nonce: &Nonce,
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
    cipher
        .decrypt(
            XNonce::from_slice(nonce.as_bytes()),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| Error::Crypto("decrypt: authentication failed".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let key = random_key();
        let nonce = Nonce::random();
        let pt = b"hello, world";
        let aad = b"record_type/Note/id/123";
        let ct = encrypt(&key, &nonce, pt, aad).unwrap();
        let out = decrypt(&key, &nonce, &ct, aad).unwrap();
        assert_eq!(out, pt);
    }

    #[test]
    fn wrong_aad_fails() {
        let key = random_key();
        let nonce = Nonce::random();
        let ct = encrypt(&key, &nonce, b"plaintext", b"aad-1").unwrap();
        assert!(decrypt(&key, &nonce, &ct, b"aad-2").is_err());
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let key = random_key();
        let nonce = Nonce::random();
        let mut ct = encrypt(&key, &nonce, b"plaintext", b"aad").unwrap();
        ct[0] ^= 0x01;
        assert!(decrypt(&key, &nonce, &ct, b"aad").is_err());
    }
}
