//! `find` — free-text search across all records.

use keepsake_core::records::Record;
use keepsake_core::session::Session;

use super::with_unlocked;

pub async fn run(path: &std::path::Path, query: String, session: &Option<Session>) -> anyhow::Result<()> {
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        for r#type in ALL_TYPES {
            for h in sess.vault.list_records(r#type)? {
                let (_h, rec) = sess.vault.get_record(h.id)?;
                if rec.matches_query(&query) {
                    println!("{}  {:>10}  {}", h.id, r#type, snippet(&rec, &query));
                }
            }
        }
        Ok(())
    })?;
    let _ = path;
    Ok(())
}

const ALL_TYPES: &[&str] = &[
    "login", "document", "identification",
    "insurance", "health", "bank_account", "credit_card",
    "investment", "income_source", "vehicle", "residence",
    "phone", "address", "contact", "subscription",
    "infrastructure", "domain", "runbook", "work_log", "note",
];

fn snippet(record: &Record, query: &str) -> String {
    let blob = record.search_blob();
    let lower = blob.to_lowercase();
    let q = query.to_lowercase();
    if let Some(pos) = lower.find(&q) {
        let start = pos.saturating_sub(20);
        let end = (pos + q.len() + 20).min(blob.len());
        let mut s = blob[start..end].to_string();
        if start > 0 { s = format!("…{s}"); }
        if end < blob.len() { s.push('…'); }
        s
    } else {
        blob.chars().take(60).collect()
    }
}
