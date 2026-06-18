//! `lock` — drop the in-memory session.

use keepsake_core::session::Session;

pub async fn run(session: &mut Option<Session>) -> anyhow::Result<()> {
    if let Some(s) = session.as_mut() {
        s.vault.lock();
        let actor = s.username.clone();
        // Vault is locked now; we cannot write the lock audit entry
        // because the vault is closed.  We just drop the session.
        let _ = actor;
    }
    *session = None;
    println!("locked");
    Ok(())
}
