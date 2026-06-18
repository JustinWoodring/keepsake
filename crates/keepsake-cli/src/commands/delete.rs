//! `delete` — delete a record by id.

use keepsake_core::audit::AuditOp;
use keepsake_core::Result;
use keepsake_core::session::Session;
use uuid::Uuid;

use super::with_unlocked_mut;

pub async fn run(
    _path: &std::path::Path,
    id: String,
    session: &mut Option<Session>,
) -> anyhow::Result<()> {
    let id = Uuid::parse_str(&id)
        .map_err(|_| anyhow::anyhow!("bad uuid: {id}"))?;
    let result = with_unlocked_mut(session, |sess| -> Result<()> {
        sess.vault.delete_record(id)?;
        sess.vault.append_audit(
            AuditOp::Delete,
            &sess.username,
            Some(&id.to_string()),
            None,
        )?;
        Ok(())
    });
    match result {
        Ok(()) => {
            println!("deleted {id}");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
