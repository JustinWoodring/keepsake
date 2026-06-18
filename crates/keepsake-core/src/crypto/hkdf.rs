//! HKDF-SHA-512 sub-key derivation.

use hkdf::Hkdf;
use sha2::Sha512;

use crate::error::{Error, Result};

use super::KEY_LEN;

/// Derive a sub-key of `output_len` bytes from `ikm` (input keying
/// material) using `salt` and `info` as HKDF parameters.
pub fn derive_subkey(ikm: &[u8], salt: &[u8], info: &[u8], output_len: usize) -> Result<Vec<u8>> {
    let hk = Hkdf::<Sha512>::new(Some(salt), ikm);
    let mut out = vec![0u8; output_len];
    hk.expand(info, &mut out)
        .map_err(|e| Error::Crypto(format!("hkdf expand: {e}")))?;
    Ok(out)
}

/// Convenience: derive a 32-byte sub-key.
pub fn derive_subkey32(ikm: &[u8], salt: &[u8], info: &[u8]) -> Result<[u8; KEY_LEN]> {
    let v = derive_subkey(ikm, salt, info, KEY_LEN)?;
    let mut out = [0u8; KEY_LEN];
    out.copy_from_slice(&v);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subkey_changes_with_info() {
        let a = derive_subkey32(b"input key material", b"salt", b"info-a").unwrap();
        let b = derive_subkey32(b"input key material", b"salt", b"info-b").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn subkey_is_deterministic() {
        let a = derive_subkey32(b"ikm", b"salt", b"info").unwrap();
        let b = derive_subkey32(b"ikm", b"salt", b"info").unwrap();
        assert_eq!(a, b);
    }
}
