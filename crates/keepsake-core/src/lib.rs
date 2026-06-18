//! `keepsake-core` — end-to-end-encrypted life organizer library.
//!
//! See `docs/threat-model.md` for the security model and `docs/crypto.md`
//! for the cryptographic design.
//!
//! High-level layout:
//!
//! * [`crypto`]    — KDF, AEAD, signing, HKDF, zeroize helpers.
//! * [`identity`] — username + password → master key, sealed vault key,
//!                  envelope keypair for server authentication.
//! * [`vault`]    — encrypted SQLite vault (records, attachments, audit
//!                  chain, sealed-keys table).
//! * [`records`]  — typed [`records::Record`] enum and per-type structs.
//! * [`doc`]      — collaborative document layer (Yrs CRDT wrapper).
//! * [`sync`]     — change feed, push/pull, vector clocks.
//! * [`transport`] — HTTP client + envelope-signed requests + long-poll.
//! * [`audit`]    — hash-chained append-only audit log.
//! * [`export`]   — `.ksk` bundle format for portable encrypted exports.
//! * [`server_admin`] — operational key material for the sync server,
//!                      separate from any vault key.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod audit;
pub mod config;
pub mod crdt;
pub mod crypto;
pub mod doc;
pub mod error;
pub mod export;
pub mod identity;
pub mod links;
pub mod records;
pub mod server_admin;
pub mod session;
pub mod sync;
pub mod transport;
pub mod vault;

pub use error::{Error, Result};
