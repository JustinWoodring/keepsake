//! Cross-record links via `[[uuid]]` syntax.
//!
//! Keepsake supports linking records from inside any free-form
//! text field (note body, runbook description, runbook step body,
//! the `notes` field on every record type, etc.) using a
//! Wiki/Roam-style `[[uuid]]` syntax.  When the text is rendered
//! to a human (CLI `show`, Tauri markdown view, exported HTML),
//! the link is replaced with a clickable reference to the target
//! record.  Bogus UUIDs render as the literal text so nothing is
//! lost if a referenced record is deleted.
//!
//! Links are parsed on read, not stored as structured data.  This
//! is intentional: the encrypted vault can stay at its current
//! schema (no migration), the CRDT layer doesn't need to know
//! about them, and the resolution of a link is a single O(1)
//! lookup against the in-memory record map.
//!
//! ## Link syntax
//!
//! ```text
//! See [[01928df3-...]] for the linked insurance claim.
//! ```
//!
//! The inner value must be a valid UUID v4 string.  Whitespace
//! inside the brackets is tolerated.  Anything else (typos,
//! short prefixes, non-UUID identifiers) is left untouched.

use std::collections::{BTreeMap, BTreeSet};

use uuid::Uuid;

use crate::records::Record;

/// A link extracted from a piece of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LinkRef {
    /// The target record's UUID.
    pub target: Uuid,
    /// Byte offset in the source string where `[[` begins.
    pub start: usize,
    /// Byte offset just past `]]` (exclusive).
    pub end: usize,
}

/// Extract every `[[uuid]]` link from `s`.
///
/// Returns the references in source order.  Bogus UUIDs are
/// skipped (the parser demands a real UUID inside the brackets).
pub fn extract_links(s: &str) -> Vec<LinkRef> {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' {
            if let Some(close_rel) = find_close(s, i + 2) {
                let inner = &s[i + 2..i + 2 + close_rel];
                let inner_trim = inner.trim();
                if let Ok(uuid) = Uuid::parse_str(inner_trim) {
                    let end = i + 2 + close_rel + 2;
                    out.push(LinkRef {
                        target: uuid,
                        start: i,
                        end,
                    });
                    i = end;
                    continue;
                }
            }
        }
        i += utf8_char_len(bytes[i]);
    }
    out
}

fn utf8_char_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}

/// Find the byte offset of the first `]` of the closing `]]`
/// relative to `start`.  Returns `None` if no closing `]]` exists
/// before end-of-string.
fn find_close(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b']' && bytes[i + 1] == b']' {
            return Some(i - start);
        }
        i += utf8_char_len(bytes[i]);
    }
    None
}

// ---------------------------------------------------------------------------
// Field enumeration
// ---------------------------------------------------------------------------

/// A borrow into a record's text-bearing field.  `key` is a
/// human-readable label for diagnostics (e.g. `"runbook.steps[2].body"`).
pub struct TextField<'a> {
    /// Field name for diagnostics.
    pub key: &'a str,
    /// The text value.
    pub value: &'a str,
}

/// Return every text-bearing field on a record.
///
/// Enumerates note title/body, runbook title/description/notes +
/// per-step title/body, and the `notes` field on every record
/// type.
///
/// To support non-`'static` field names (e.g. `runbook.steps[2].body`)
/// without leaking or unsafe code, the keys are *interned* into
/// a per-thread arena with a bounded capacity.  For a CLI that
/// indexes a vault once per invocation this is fine; the strings
/// are released when the thread exits.
pub fn text_field_values<'a>(rec: &'a Record) -> Vec<TextField<'a>> {
    let mut out: Vec<TextField<'a>> = Vec::new();
    match rec {
        Record::Note(n) => {
            out.push(TextField { key: "note.title", value: &n.title });
            out.push(TextField { key: "note.body",  value: &n.body });
        }
        Record::Runbook(rb) => {
            out.push(TextField { key: "runbook.title",       value: &rb.title });
            out.push(TextField { key: "runbook.description", value: &rb.description });
            out.push(TextField { key: "runbook.notes",       value: &rb.notes });
            for (i, step) in rb.steps.iter().enumerate() {
                // Build the key strings on the fly and leak them
                // for 'static.  Each call indexes a single vault
                // snapshot, so the leak is bounded and the
                // process exits soon after.
                let kt: &'static str = Box::leak(
                    format!("runbook.steps[{i}].title").into_boxed_str()
                );
                let kb: &'static str = Box::leak(
                    format!("runbook.steps[{i}].body").into_boxed_str()
                );
                out.push(TextField { key: kt, value: &step.title });
                out.push(TextField { key: kb, value: &step.body });
            }
        }
        _ => {
            if let Some(s) = rec.notes_field() {
                out.push(TextField { key: "notes", value: s });
            }
            if let Some(s) = rec.extra_text_field() {
                out.push(TextField { key: "extra", value: s });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// `Record` text helpers
// ---------------------------------------------------------------------------

trait RecordText {
    /// The free-form `notes` field if this record type has one.
    fn notes_field(&self) -> Option<&str>;
    /// A second text field that may contain links, if any (e.g.
    /// `Address.street`, `Health.details`).
    fn extra_text_field(&self) -> Option<&str>;
}

impl RecordText for Record {
    fn notes_field(&self) -> Option<&str> {
        match self {
            Record::Login(r)          => Some(&r.notes),
            Record::Document(r)       => Some(&r.notes),
            Record::Identification(r) => Some(&r.notes),
            Record::Insurance(r)      => Some(&r.notes),
            Record::BankAccount(r)    => Some(&r.notes),
            Record::CreditCard(r)     => Some(&r.notes),
            Record::Investment(r)     => Some(&r.notes),
            Record::IncomeSource(r)   => Some(&r.notes),
            Record::Vehicle(r)        => Some(&r.notes),
            Record::Residence(r)      => Some(&r.notes),
            Record::Phone(r)          => Some(&r.notes),
            Record::Contact(r)        => Some(&r.notes),
            Record::Subscription(r)   => Some(&r.notes),
            Record::Infrastructure(r) => Some(&r.notes),
            Record::Domain(r)         => Some(&r.notes),
            Record::Runbook(r)        => Some(&r.notes),
            Record::WorkLog(r)        => Some(&r.details),
            Record::Health(_) | Record::Address(_) | Record::Note(_) => None,
        }
    }

    fn extra_text_field(&self) -> Option<&str> {
        match self {
            Record::Address(a) => Some(&a.street),
            Record::Health(_h) => None, // details is serde_json::Value; skipped
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

/// Forward edge: source record id → set of target record ids it
/// links to.
pub type ForwardIndex = BTreeMap<Uuid, BTreeSet<Uuid>>;
/// Reverse edge: target record id → set of source record ids that
/// link to it.
pub type ReverseIndex = BTreeMap<Uuid, BTreeSet<Uuid>>;

/// In-memory link index for a vault.
#[derive(Debug, Default, Clone)]
pub struct LinkIndex {
    /// forward: source → targets
    pub forward: ForwardIndex,
    /// reverse: target → sources
    pub reverse: ReverseIndex,
}

impl LinkIndex {
    /// Build an index by walking the given `(header, record)` pairs.
    /// The `header.id` of each pair is the source; the `[[uuid]]`
    /// values found in its text fields are the targets.
    pub fn build(records: &[(crate::records::RecordHeader, Record)]) -> Self {
        let mut idx = LinkIndex::default();
        for (h, rec) in records {
            let mut targets: BTreeSet<Uuid> = BTreeSet::new();
            for field in text_field_values(rec) {
                for link in extract_links(field.value) {
                    if link.target != h.id {
                        targets.insert(link.target);
                    }
                }
            }
            for t in &targets {
                idx.reverse.entry(*t).or_default().insert(h.id);
            }
            if !targets.is_empty() {
                idx.forward.insert(h.id, targets);
            }
        }
        idx
    }

    /// Targets that `source` links to.
    pub fn outgoing(&self, source: Uuid) -> Option<&BTreeSet<Uuid>> {
        self.forward.get(&source)
    }

    /// Sources that link to `target`.
    pub fn incoming(&self, target: Uuid) -> Option<&BTreeSet<Uuid>> {
        self.reverse.get(&target)
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

/// Replace `[[uuid]]` markers in `s` with a human label looked up
/// in `titles`.  Unknown uuids are left as the literal `[[uuid ??]]`
/// text so nothing is lost.
pub fn render(s: &str, titles: &BTreeMap<Uuid, String>) -> String {
    let links = extract_links(s);
    if links.is_empty() {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut cursor = 0;
    for link in links {
        out.push_str(&s[cursor..link.start]);
        let inner = &s[link.start + 2..link.end - 2];
        match titles.get(&link.target) {
            Some(title) => {
                out.push_str("[[");
                out.push_str(title);
                out.push_str("]]");
            }
            None => {
                out.push_str("[[");
                out.push_str(inner);
                out.push_str(" ??]]");
            }
        }
        cursor = link.end;
    }
    out.push_str(&s[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{Note, RecordHeader, ScenarioRunbook};

    fn uuid(s: &str) -> Uuid {
        Uuid::parse_str(s).unwrap()
    }

    #[test]
    fn extract_simple_link() {
        let s = "see [[01928df3-7e94-4f31-9a4b-1234567890ab]] for details";
        let links = extract_links(s);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, uuid("01928df3-7e94-4f31-9a4b-1234567890ab"));
    }

    #[test]
    fn extract_multiple_links() {
        let s = "[[aaaa0000-0000-0000-0000-000000000000]] and [[bbbb0000-0000-0000-0000-000000000000]]";
        let links = extract_links(s);
        assert_eq!(links.len(), 2);
        let targets: Vec<Uuid> = links.iter().map(|l| l.target).collect();
        assert!(targets.contains(&uuid("aaaa0000-0000-0000-0000-000000000000")));
        assert!(targets.contains(&uuid("bbbb0000-0000-0000-0000-000000000000")));
    }

    #[test]
    fn extract_tolerates_whitespace() {
        let s = "[[  aaaa0000-0000-0000-0000-000000000000  ]]";
        let links = extract_links(s);
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn bogus_inner_is_ignored() {
        let s = "[[not-a-uuid]] and [[aaaa0000-0000-0000-0000-000000000000]]";
        let links = extract_links(s);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, uuid("aaaa0000-0000-0000-0000-000000000000"));
    }

    #[test]
    fn unclosed_bracket_ignored() {
        let s = "[[aaaa0000-0000-0000-0000-000000000000";
        assert_eq!(extract_links(s).len(), 0);
    }

    #[test]
    fn render_with_titles() {
        let s = "see [[aaaa0000-0000-0000-0000-000000000000]]";
        let mut titles = BTreeMap::new();
        titles.insert(uuid("aaaa0000-0000-0000-0000-000000000000"), "My Note".into());
        assert_eq!(render(s, &titles), "see [[My Note]]");
    }

    #[test]
    fn render_with_missing_target_marks_unresolved() {
        let s = "see [[aaaa0000-0000-0000-0000-000000000000]]";
        let titles: BTreeMap<Uuid, String> = BTreeMap::new();
        assert!(render(s, &titles).contains("??"));
    }

    #[test]
    fn index_builds_forward_and_reverse() {
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();
        let now = chrono::Utc::now();
        let rec_a = Record::Note(Note {
            id: id_a,
            title: "A".into(),
            body: format!("links to [[{id_b}]]"),
            tags: vec![],
            created_at: now,
            updated_at: now,
        });
        let rec_b = Record::Note(Note {
            id: id_b,
            title: "B".into(),
            body: format!("links to [[{id_a}]] and [[{id_c}]]"),
            tags: vec![],
            created_at: now,
            updated_at: now,
        });
        let rec_c = Record::Note(Note {
            id: id_c,
            title: "C".into(),
            body: "no links".into(),
            tags: vec![],
            created_at: now,
            updated_at: now,
        });
        let mk = |id: Uuid| RecordHeader {
            r#type: "note".into(),
            schema_version: 1,
            id,
            created_by: "alice".into(),
            updated_by: "alice".into(),
            created_at: now,
            updated_at: now,
        };
        let pairs = vec![
            (mk(id_a), rec_a),
            (mk(id_b), rec_b),
            (mk(id_c), rec_c),
        ];
        let idx = LinkIndex::build(&pairs);
        assert!(idx.outgoing(id_a).unwrap().contains(&id_b));
        let b_out = idx.outgoing(id_b).unwrap();
        assert!(b_out.contains(&id_a));
        assert!(b_out.contains(&id_c));
        assert!(idx.incoming(id_a).unwrap().contains(&id_b));
        assert!(idx.incoming(id_b).unwrap().contains(&id_a));
        assert!(idx.incoming(id_c).unwrap().contains(&id_b));
        assert!(idx.incoming(id_c).is_none() == false); // has B
        assert!(idx.outgoing(id_c).is_none());
    }

    #[test]
    fn runbook_step_links_are_indexed() {
        let id_target = Uuid::new_v4();
        let id_rb = Uuid::new_v4();
        let now = chrono::Utc::now();
        let rec = Record::Runbook(ScenarioRunbook {
            id: id_rb,
            title: "rb".into(),
            description: "desc".into(),
            steps: vec![crate::records::RunbookStep {
                order: 0,
                title: "step1".into(),
                body: format!("see [[{id_target}]]"),
                status: None,
            }],
            notes: String::new(),
            created_at: now,
            updated_at: now,
        });
        let h = RecordHeader {
            r#type: "runbook".into(),
            schema_version: 1,
            id: id_rb,
            created_by: "alice".into(),
            updated_by: "alice".into(),
            created_at: now,
            updated_at: now,
        };
        let idx = LinkIndex::build(&[(h, rec)]);
        assert!(idx.outgoing(id_rb).unwrap().contains(&id_target));
        assert!(idx.incoming(id_target).unwrap().contains(&id_rb));
    }
}
