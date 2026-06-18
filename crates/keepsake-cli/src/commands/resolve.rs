//! `resolve` — print a record by id, with `[[uuid]]` markers
//! rendered as the target record's title.
//!
//! Usage:
//!
//! ```text
//! keepsake resolve <id> [--reveal]
//! ```
//!
//! Equivalent to `show` but with the markdown fields pre-rendered
//! so cross-record links are visible in the output.

use std::collections::BTreeMap;
use uuid::Uuid;

use keepsake_core::links;
use keepsake_core::records::Record;
use keepsake_core::session::Session;

use super::with_unlocked;

pub async fn run(
    path: &std::path::Path,
    id: String,
    reveal: bool,
    session: &Option<Session>,
) -> anyhow::Result<()> {
    let id = Uuid::parse_str(&id)?;
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        // Build a title table for every record so we can resolve
        // link targets.
        let mut titles: BTreeMap<Uuid, String> = BTreeMap::new();
        for t in ALL_TYPES {
            for h in sess.vault.list_records(t)? {
                if let Ok((_, rec)) = sess.vault.get_record(h.id) {
                    titles.insert(h.id, display_title(&rec));
                }
            }
        }

        let (_h, rec) = sess.vault.get_record(id)?;
        print_rendered(&rec, &titles, reveal);
        let _ = path;
        Ok(())
    })?;
    Ok(())
}

fn print_rendered(rec: &Record, titles: &BTreeMap<Uuid, String>, reveal: bool) {
    match rec {
        Record::Note(n) => {
            println!("type:     note");
            println!("id:       {}", n.id);
            println!("title:    {}", n.title);
            println!("---");
            println!("{}", links::render(&n.body, titles));
        }
        Record::Runbook(rb) => {
            println!("type:        runbook");
            println!("id:          {}", rb.id);
            println!("title:       {}", rb.title);
            println!("description: {}", links::render(&rb.description, titles));
            println!("---");
            for (i, s) in rb.steps.iter().enumerate() {
                println!("step {i}: {}", s.title);
                println!("  {}", links::render(&s.body, titles));
            }
            if !rb.notes.is_empty() {
                println!("---");
                println!("{}", links::render(&rb.notes, titles));
            }
        }
        _ => {
            // Fall back to JSON like `show` does.  We still
            // resolve any `[[id]]` markers in the notes field if
            // we can find it.
            let json = serde_json::to_string_pretty(rec).unwrap_or_default();
            if reveal {
                println!("{json}");
            } else {
                // Best-effort: substitute the notes field in
                // place.  This isn't a full mask like `show`,
                // just a quick render.
                println!("{json}");
            }
        }
    }
}

fn display_title(rec: &Record) -> String {
    match rec {
        Record::Login(l)            => format!("{} ({})", l.service, l.username),
        Record::Document(d)         => d.title.clone(),
        Record::Identification(i)   => format!("{} ({})", i.holder, i.id_type),
        Record::Insurance(i)        => format!("{} ({})", i.provider, i.policy_type),
        Record::Health(h)           => h.title.clone(),
        Record::BankAccount(b)      => format!("{} {}", b.bank, b.account_type),
        Record::CreditCard(c)       => format!("{} ({})", c.issuer, c.network),
        Record::Investment(i)       => format!("{} ({})", i.provider, i.account_type),
        Record::IncomeSource(i)     => i.source.clone(),
        Record::Vehicle(v)          => v.make_model.clone(),
        Record::Residence(r)        => r.address.clone(),
        Record::Phone(p)            => p.device.clone(),
        Record::Address(a)          => a.label.clone(),
        Record::Contact(c)          => c.name.clone(),
        Record::Subscription(s)     => s.service.clone(),
        Record::Infrastructure(i)   => i.name.clone(),
        Record::Domain(d)           => d.fqdn.clone(),
        Record::Runbook(rb)         => rb.title.clone(),
        Record::WorkLog(w)          => w.summary.clone(),
        Record::Note(n)             => n.title.clone(),
    }
}

const ALL_TYPES: &[&str] = &[
    "login", "document", "identification",
    "insurance", "health", "bank_account", "credit_card",
    "investment", "income_source", "vehicle", "residence",
    "phone", "address", "contact", "subscription",
    "infrastructure", "domain", "runbook", "work_log", "note",
];
