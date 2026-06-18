//! `audit` — read or verify the audit chain.

use keepsake_core::session::Session;

use super::with_unlocked;

pub async fn run(path: &std::path::Path, verify: bool, session: &Option<Session>) -> anyhow::Result<()> {
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        if verify {
            // Strict verify: the whole chain must verify, else error.
            match sess.vault.verify_audit_chain() {
                Ok(()) => {
                    let n = sess.vault.read_audit()?.len();
                    println!("audit chain verified: {} entries", n);
                    Ok(())
                }
                Err(keepsake_core::Error::AuditTampered(seq)) => {
                    eprintln!("audit chain verification failed at entry {}", seq);
                    Err(keepsake_core::Error::AuditTampered(seq))
                }
                Err(e) => Err(e),
            }
        } else {
            // Listing mode: print whatever is on disk, no chain
            // check.  This is the right thing to do when an old
            // entry was written by a different version of the
            // code (hash function changed, but the data is
            // still readable).
            let entries = sess.vault.read_audit()?;
            println!("seq  op            actor    target          ts");
            for e in entries {
                println!(
                    "{:>4}  {:>12}  {:<8}  {:<14}  {}",
                    e.seq,
                    format!("{:?}", e.op),
                    e.actor,
                    e.target_id.as_deref().unwrap_or(""),
                    e.ts.to_rfc3339(),
                );
            }
            Ok(())
        }
    })?;
    let _ = path;
    Ok(())
}
