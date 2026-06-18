//! `audit reset` — drop untrusted legacy entries and re-chain.
//!
//! Reads every audit entry, finds the first one whose stored
//! hash doesn't match the current `entry_hash` function (i.e.
//! it was written by an older version of the code), drops
//! every entry before it, and rewrites the survivors with new
//! seq numbers starting at 1.

use keepsake_core::session::Session;

use super::with_unlocked;

pub async fn run(
    path: &std::path::Path,
    yes: bool,
    session: &Option<Session>,
) -> anyhow::Result<()> {
    if !yes {
        eprintln!(
            "This will drop every audit entry before the first one that\n\
             doesn't match the current hash format, then re-chain the\n\
             rest.  Destructive; cannot be undone.\n\n\
             Re-run with --yes to confirm."
        );
        anyhow::bail!("aborted; pass --yes to confirm");
    }
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        let dropped = sess.vault.rewrite_audit_chain()?;
        if dropped == 0 {
            println!("audit chain was already valid; no changes made");
        } else {
            println!("dropped {} legacy entries and re-chained the rest", dropped);
        }
        Ok(())
    })?;
    let _ = path;
    Ok(())
}
