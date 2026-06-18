//! Re-export of the `zeroize` crate.  Centralised so other modules
//! don't have to know which exact version is in use.

pub use ::zeroize::Zeroizing;

/// Standard symmetric key length in bytes.
pub const KEY_LEN: usize = 32;
