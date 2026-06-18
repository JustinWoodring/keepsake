//! `show` — show a record by id.

use uuid::Uuid;

use keepsake_core::records::Record;
use keepsake_core::session::Session;

use super::with_unlocked;

pub async fn run(path: &std::path::Path, id: String, reveal: bool, session: &Option<Session>) -> anyhow::Result<()> {
    let id = Uuid::parse_str(&id)?;
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        let (_h, rec) = sess.vault.get_record(id)?;
        let json = serde_json::to_string_pretty(&rec)?;
        if reveal {
            println!("{json}");
        } else {
            println!("{}", mask(&rec, json));
        }
        Ok(())
    })?;
    let _ = path;
    Ok(())
}

/// Mask fields whose names suggest they're sensitive.
fn mask(record: &Record, json: String) -> String {
    let sensitive = match record {
        Record::Login(_) => &["password", "totp_secret", "recovery_codes"][..],
        Record::Identification(_) => &["number"][..],
        Record::BankAccount(_) => &[
            "account_number",
            "routing_number",
            "online_username",
        ][..],
        Record::CreditCard(_) => &["card_number", "cvv", "pin", "expiration"][..],
        Record::Phone(_) => &["pin", "account_number"][..],
        _ => &[][..],
    };
    if sensitive.is_empty() {
        return json;
    }
    let mut out = json;
    for key in sensitive {
        let needle = format!("\"{key}\":");
        if let Some(start) = out.find(&needle) {
            if let Some(rest_start) = out[start + needle.len()..].find('"') {
                let value_start = start + needle.len() + rest_start + 1;
                if let Some(rel_end) = out[value_start..].find('"') {
                    let value_end = value_start + rel_end;
                    out.replace_range(value_start..value_end, "***");
                }
            }
        }
    }
    out
}
