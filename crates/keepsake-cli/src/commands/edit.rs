//! `edit` — re-prompt for fields of a record and write it back.
//!
//! In v1 this re-runs the same prompts as `add` (with the
//! current values as defaults) and writes the result.  All 20
//! record types are supported because we delegate to
//! `commands::add::prompt_for_with_defaults`.

use chrono::Utc;

use keepsake_core::audit::AuditOp;
use keepsake_core::session::Session;
use keepsake_core::Result;

use super::with_unlocked;
use super::with_unlocked_mut;
use uuid::Uuid;

use crate::commands::add;

pub async fn run(path: &std::path::Path, id: String, session: &mut Option<Session>) -> anyhow::Result<()> {
    let id = Uuid::parse_str(&id)?;
    let r#type = with_unlocked(session, |sess| -> Result<String> {
        let (header, _rec) = sess.vault.get_record(id)?;
        Ok(header.r#type.clone())
    })?;
    with_unlocked_mut(session, |sess| -> Result<()> {
        let (_header, record) = sess.vault.get_record(id)?;
        let new_record = add::prompt_for_with_defaults(&r#type, &record)
            .map_err(|e| keepsake_core::Error::Invalid(e.to_string()))?;
        new_record.validate().map_err(|e| keepsake_core::Error::Invalid(e.to_string()))?;
        let mut new_header = sess.vault.get_record(id)?.0;
        new_header.updated_at = Utc::now();
        new_header.updated_by = sess.username.clone();
        sess.vault.put_record(&new_header, &new_record)?;
        sess.vault.append_audit(
            AuditOp::Update,
            &sess.username,
            Some(&id.to_string()),
            Some(&r#type),
        )?;
        Ok(())
    })?;
    println!("updated {id}");
    let _ = path;
    Ok(())
}
