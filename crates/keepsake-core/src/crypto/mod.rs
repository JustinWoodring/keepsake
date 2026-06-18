//! Cryptographic primitives used by the vault.
//!
//! * Argon2id for password-based key derivation.
//! * XChaCha20-Poly1305 for authenticated encryption.
//! * Ed25519 for envelope authentication (server requests).
//! * HKDF-SHA-512 for sub-key derivation.
//! * blake3 for the audit chain and content addressing.

pub mod aead;
pub mod hkdf;
pub mod kdf;
pub mod sign;
pub mod zeroize;

pub use aead::{decrypt, encrypt, AeadKey, Nonce, TAG_LEN};
pub use hkdf::derive_subkey;
pub use kdf::{derive_master_key, KdfParams, MasterKey};
pub use sign::{verify_signature, Signature, SigningKey, VerifyingKey, ED25519_PK_LEN, ED25519_SIG_LEN};
pub use zeroize::{Zeroizing, KEY_LEN};
