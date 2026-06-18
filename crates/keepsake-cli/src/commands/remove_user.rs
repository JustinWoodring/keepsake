//! `remove-user` — remove a user from this device's vault.

use keepsake_core::Result;
use keepsake_core::session::Session;

use super::with_unlocked_mut;

pub async fn run(
    _path: &std::path::Path,
    username: Option<String>,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let username = match username {
        Some(u) if !u.trim().is_empty() => u,
        _ => dialoguer::Input::new()
            .with_prompt("username to remove")
            .interact_text()?,
    };
    let result = with_unlocked_mut(session, |sess| -> Result<()> {
        if sess.username == username {
            return Err(keepsake_core::Error::Invalid(
                "cannot remove the currently-logged-in user; lock first".into(),
            ));
        }
        let cur = sess.username.clone();
        sess.vault.delete_sealed_key(&username)?;
        use keepsake_core::audit::AuditOp;
        sess.vault.append_audit(
            AuditOp::RemoveUser,
            &cur,
            Some(&username),
            None,
        )?;
        Ok(())
    });
    match result {
        Ok(()) => {
            println!("removed user {username}");
            Ok(())
        }
        Err(e) => Err(e),
    }
}
