//! `export` — write a `.ksk` bundle.

use keepsake_core::export::{build_bundle, write_bundle};
use keepsake_core::session::Session;

use super::with_unlocked;

pub async fn run(path: &std::path::Path, out: std::path::PathBuf, session: &Option<Session>) -> anyhow::Result<()> {
    let passphrase = rpassword::prompt_password("export passphrase: ")?;
    let confirm = rpassword::prompt_password("confirm:           ")?;
    if passphrase != confirm {
        return Err(anyhow::anyhow!("passphrases do not match"));
    }
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        let row = sess.vault.get_sealed_key(&sess.username)?
            .ok_or_else(|| keepsake_core::Error::NotFound(sess.username.clone()))?;
        let bundle = build_bundle(&sess.vault, &sess.master, &row, passphrase.as_bytes())?;
        write_bundle(&bundle, &out)?;
        sess.vault.append_audit(
            keepsake_core::audit::AuditOp::Export,
            &sess.username,
            Some(&out.display().to_string()),
            None,
        )?;
        Ok(())
    })?;
    println!("exported to {}", out.display());
    let _ = path;
    Ok(())
}
