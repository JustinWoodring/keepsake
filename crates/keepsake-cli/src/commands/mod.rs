//! Subcommand implementations.  Each subcommand takes a
//! `&mut Option<Session>` from the CLI's main loop.  Locking
//! helpers below ensure the vault is unlocked.

use keepsake_core::Result;

/// Run `f` with a mutable view of the session, returning an
/// error if the vault is locked.
pub fn with_unlocked_mut<R>(
    session: &mut Option<keepsake_core::session::Session>,
    f: impl FnOnce(&mut keepsake_core::session::Session) -> Result<R>,
) -> anyhow::Result<R> {
    let s = session.as_mut().ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
    f(s).map_err(Into::into)
}

/// Run `f` with a read-only view of the session.
pub fn with_unlocked<R>(
    session: &Option<keepsake_core::session::Session>,
    f: impl FnOnce(&keepsake_core::session::Session) -> Result<R>,
) -> anyhow::Result<R> {
    let s = session.as_ref().ok_or_else(|| anyhow::anyhow!("vault is locked"))?;
    f(s).map_err(Into::into)
}

pub mod add;
pub mod add_user;
pub mod audit;
pub mod audit_reset;
pub mod change_password;
pub mod default_path;
pub mod delete;
pub mod edit;
pub mod export;
pub mod find;
pub mod import;
pub mod init;
pub mod links;
pub mod list;
pub mod list_users;
pub mod lock;
pub mod remove_user;
pub mod repl;
pub mod resolve;
pub mod show;
pub mod status;
pub mod sync;
pub mod unlock;
pub mod whoami;
