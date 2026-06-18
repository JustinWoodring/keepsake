//! `list` — list records of a given type.

use comfy_table::Table;

use keepsake_core::session::Session;

use super::with_unlocked;

pub async fn run(path: &std::path::Path, r#type: String, session: &Option<Session>) -> anyhow::Result<()> {
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        let headers = sess.vault.list_records(&r#type)?;
        let mut t = Table::new();
        t.set_header(vec!["id", "type", "updated_by", "updated_at"]);
        for h in headers {
            t.add_row(vec![
                h.id.to_string(),
                h.r#type,
                h.updated_by,
                h.updated_at.to_rfc3339(),
            ]);
        }
        println!("{t}");
        Ok(())
    })?;
    let _ = path;
    Ok(())
}
