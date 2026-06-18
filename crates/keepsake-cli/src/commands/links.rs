//! `links` — show the cross-record links attached to a record.
//!
//! Usage:
//!
//! ```text
//! keepsake links <id>              # both forward and reverse
//! keepsake links <id> --out        # only forward (this record's targets)
//! keepsake links <id> --in         # only reverse (backlinks)
//! ```
//!
//! Links are `[[uuid]]` references in any text field on the
//! record (note body, runbook description, runbook step body, the
//! `notes` field on any record type, etc.).  See
//! `keepsake_core::links` for the parser.

use uuid::Uuid;

use keepsake_core::links::LinkIndex;
use keepsake_core::records::Record;
use keepsake_core::session::Session;

use super::with_unlocked;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Both,
    Out,
    In,
}

pub async fn run(
    path: &std::path::Path,
    id: String,
    direction: String,
    session: &Option<Session>,
) -> anyhow::Result<()> {
    let id = Uuid::parse_str(&id)?;
    let dir = match direction.as_str() {
        "out"  => Direction::Out,
        "in"   => Direction::In,
        "both" | "" => Direction::Both,
        other  => anyhow::bail!("unknown direction '{other}'; expected out, in, or both"),
    };
    with_unlocked(session, |sess| -> keepsake_core::Result<()> {
        // Build the index over every record.  For small vaults
        // this is fine; for very large vaults a future patch
        // adds a per-record cached link table.
        let mut all: Vec<(keepsake_core::records::RecordHeader, Record)> = Vec::new();
        for t in ALL_TYPES {
            for h in sess.vault.list_records(t)? {
                if let Ok((_, rec)) = sess.vault.get_record(h.id) {
                    all.push((h, rec));
                }
            }
        }
        let idx = LinkIndex::build(&all);

        let title = |uuid: Uuid| -> String {
            all.iter()
                .find(|(h, _)| h.id == uuid)
                .map(|(h, r)| format!("{}  ({})", display_title(r), h.r#type))
                .unwrap_or_else(|| format!("<missing>  ({uuid})"))
        };

        if matches!(dir, Direction::Out | Direction::Both) {
            println!("outgoing (this record → targets):");
            match idx.outgoing(id) {
                None => println!("  (none)"),
                Some(set) => {
                    let mut v: Vec<Uuid> = set.iter().copied().collect();
                    v.sort();
                    for t in v {
                        println!("  - {}  {}", t, title(t));
                    }
                }
            }
        }
        if matches!(dir, Direction::In | Direction::Both) {
            if matches!(dir, Direction::Both) { println!(); }
            println!("incoming (backlinks: sources that point to this record):");
            match idx.incoming(id) {
                None => println!("  (none)"),
                Some(set) => {
                    let mut v: Vec<Uuid> = set.iter().copied().collect();
                    v.sort();
                    for t in v {
                        println!("  - {}  {}", t, title(t));
                    }
                }
            }
        }

        let _ = path;
        Ok(())
    })?;
    Ok(())
}

fn display_title(rec: &Record) -> String {
    match rec {
        Record::Login(l) => format!("{} ({})", l.service, l.username),
        Record::Document(d) => d.title.clone(),
        Record::Identification(i) => format!("{} ({})", i.holder, i.id_type),
        Record::Insurance(i) => format!("{} ({})", i.provider, i.policy_type),
        Record::Health(h) => h.title.clone(),
        Record::BankAccount(b) => format!("{} {}", b.bank, b.account_type),
        Record::CreditCard(c) => format!("{} ({})", c.issuer, c.network),
        Record::Investment(i) => format!("{} ({})", i.provider, i.account_type),
        Record::IncomeSource(i) => i.source.clone(),
        Record::Vehicle(v) => v.make_model.clone(),
        Record::Residence(r) => r.address.clone(),
        Record::Phone(p) => p.device.clone(),
        Record::Address(a) => a.label.clone(),
        Record::Contact(c) => c.name.clone(),
        Record::Subscription(s) => s.service.clone(),
        Record::Infrastructure(i) => i.name.clone(),
        Record::Domain(d) => d.fqdn.clone(),
        Record::Runbook(rb) => rb.title.clone(),
        Record::WorkLog(w) => w.summary.clone(),
        Record::Note(n) => n.title.clone(),
    }
}

const ALL_TYPES: &[&str] = &[
    "login", "document", "identification",
    "insurance", "health", "bank_account", "credit_card",
    "investment", "income_source", "vehicle", "residence",
    "phone", "address", "contact", "subscription",
    "infrastructure", "domain", "runbook", "work_log", "note",
];
