//! User identity, master-key derivation, sealed vault key, and
//! envelope (server-authentication) keypair.

pub mod envelope_key;
pub mod password;
pub mod vault_key;

pub use crate::crypto::MasterKey;
pub use envelope_key::{EnvelopeKey, EnvelopePublicKey};
pub use password::{password_to_master_key, Salt};
pub use vault_key::{seal_vault_key, unseal_vault_key, SealedVaultKey, VaultKey};
