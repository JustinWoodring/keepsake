//! Top-level error type for `keepsake-core`.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("vault error: {0}")]
    Vault(String),

    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("identity error: {0}")]
    Identity(String),

    #[error("record error: {0}")]
    Record(String),

    #[error("sync error: {0}")]
    Sync(String),

    #[error("transport error: {0}")]
    Transport(String),

    #[error("audit chain verification failed at entry {0}")]
    AuditTampered(u64),

    #[error("export error: {0}")]
    Export(String),

    #[error("invalid argument: {0}")]
    Invalid(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("locked — vault is not unlocked")]
    Locked,
}
