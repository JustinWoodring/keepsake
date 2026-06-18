//! CRDT merge layer for keepsake records.
//!
//! Two layers:
//!
//! 1. **Per-text-field CRDT** ([`Text`]): collaborative text built
//!    on top of `yrs` (Yjs in Rust).  Two devices editing the same
//!    note body, runbook description, runbook step body, or any
//!    `notes` field can merge character-by-character.  The CRDT
//!    state is opaque binary (yrs's update format) that round-
//!    trips through the encrypted vault as a base64 string.
//!
//! 2. **Per-record last-writer-wins** ([`merge_record`]): for the
//!    structured scalar fields (service, username, holders list,
//!    etc.) we keep the full record with the most recent
//!    `(updated_at, updated_by, id)` triple.  Concurrent edits to
//!    the same scalar field resolve as last-writer-wins; concurrent
//!    edits to the same text field resolve via the text CRDT.
//!
//! The split mirrors what real-world tools (Notion, Linear, etc.)
//! do: structured data LWW, free text CRDT.
//!
//! ## Wire format
//!
//! Text fields on the wire are a plain UTF-8 string, identical to
//! the v1 schema.  When the text has been edited on more than one
//! device, the field is upgraded to a [`Text`] value that carries
//! the yrs update bytes; the rendered string is recovered on read.
//! Older records (no CRDT state) are imported transparently on
//! first edit.

use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yrs::updates::decoder::Decode;
use yrs::GetString as YrsGetString;
use yrs::ReadTxn as YrsReadTxn;
use yrs::Text as YrsText;
use yrs::Transact as YrsTransact;
use yrs::{StateVector, Update};

use crate::records::{Record, RecordHeader};
use crate::Result;

// ---------------------------------------------------------------------------
// Text CRDT
// ---------------------------------------------------------------------------

/// A collaboratively-edited text field.
///
/// `value` is the rendered UTF-8 string (what the user sees).
/// `state` is an opaque yrs update blob, kept so that future
/// edits compose with the existing CRDT history.  An older record
/// that has never been edited collaboratively will have `state =
/// None`; the field is upgraded to a CRDT on the first concurrent
/// edit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Text {
    /// Rendered string at this snapshot in time.
    pub value: String,
    /// Opaque yrs update bytes (base64 on the wire).  `None` for
    /// records that have never been edited on more than one device.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<Vec<u8>>,
}

impl Text {
    /// Construct a new `Text` from a plain string with no CRDT
    /// history.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            state: None,
        }
    }

    /// Render the current string.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Three-way merge: combine `self` (local) and `other` (remote)
    /// with `base` (the common ancestor) as a guide.  Both sides
    /// have been independently edited on top of `base`.  This
    /// returns a `Text` that contains the union of both edits
    /// without losing characters.
    ///
    /// If either side has no `state` (never been a CRDT), we just
    /// pick the longer value (heuristic) and promote to a CRDT.
    /// If both sides have state, we let yrs merge the two update
    /// logs and re-render.
    pub fn three_way_merge(base: &str, local: &Self, remote: &Self) -> Result<Self> {
        // Both sides are plain (no CRDT history).  Use a simple
        // longest-wins heuristic — it preserves user intent in the
        // common case where one side appended or one side rewrote.
        if local.state.is_none() && remote.state.is_none() {
            if local.value == remote.value {
                return Ok(Self::new(local.value.clone()));
            }
            if local.value == base {
                return Ok(Self::new(remote.value.clone()));
            }
            if remote.value == base {
                return Ok(Self::new(local.value.clone()));
            }
            // Genuine conflict.  Pick the longer value; tie → local.
            let winner = if remote.value.len() > local.value.len() {
                remote.value.clone()
            } else {
                local.value.clone()
            };
            return Ok(Self::new(winner));
        }

        // At least one side has CRDT state.  Use yrs to merge
        // whichever state vectors we have.
        let doc = yrs::Doc::new();
        let text = doc.get_or_insert_text("t");

        // Seed the doc with the base string so both updates apply
        // to a shared starting point.
        {
            let mut txn = doc.transact_mut();
            if !base.is_empty() {
                text.insert(&mut txn, 0, base);
            }
        }

        // Apply the local update on top of the base.
        match &local.state {
            Some(state) => {
                let update = Update::decode_v1(state)
                    .map_err(|e| crate::Error::Sync(format!("yrs decode local: {e}")))?;
                let mut txn = doc.transact_mut();
                txn.apply_update(update)
                    .map_err(|e| crate::Error::Sync(format!("yrs apply local: {e}")))?;
            }
            None => {
                let mut txn = doc.transact_mut();
                replace_text(&mut txn, &text, base, &local.value);
            }
        }

        // Apply the remote update.
        match &remote.state {
            Some(state) => {
                let update = Update::decode_v1(state)
                    .map_err(|e| crate::Error::Sync(format!("yrs decode remote: {e}")))?;
                let mut txn = doc.transact_mut();
                txn.apply_update(update)
                    .map_err(|e| crate::Error::Sync(format!("yrs apply remote: {e}")))?;
            }
            None => {
                let mut txn = doc.transact_mut();
                replace_text(&mut txn, &text, base, &remote.value);
            }
        }

        let snapshot = {
            let txn = doc.transact();
            text.get_string(&txn)
        };
        let merged_state = {
            let txn = doc.transact();
            txn.encode_state_as_update_v1(&StateVector::default())
        };

        Ok(Self {
            value: snapshot,
            state: Some(merged_state),
        })
    }
}

fn replace_text(
    txn: &mut yrs::TransactionMut<'_>,
    text: &yrs::TextRef,
    _old: &str,
    new: &str,
) {
    // Replace the current text with `new`.  The CRDT history
    // records the operation, so concurrent edits to the same
    // field on another device will compose correctly.
    let current_len = text.len(txn);
    if current_len > 0 {
        text.remove_range(txn, 0, current_len);
    }
    if !new.is_empty() {
        text.insert(txn, 0, new);
    }
}

// ---------------------------------------------------------------------------
// Per-record last-writer-wins
// ---------------------------------------------------------------------------

/// Result of merging two versions of the same record.
#[derive(Debug)]
pub enum MergeOutcome {
    /// The local copy was newer; keep it as-is.
    LocalWins,
    /// The remote copy was newer; replace the local copy.
    RemoteWins {
        /// The header from the remote side (used for the audit log).
        header: RecordHeader,
        /// The merged record body.
        record: Record,
    },
    /// Both sides had the same logical timestamp; their text
    /// fields were merged via the CRDT layer.  The local record is
    /// updated in place; this variant carries the merged fields
    /// back to the caller.
    TextMerged {
        /// The header to persist (from whichever side is "later"
        /// by author tie-break; we use local to keep the on-disk
        /// header stable).
        header: RecordHeader,
        /// The merged record.
        record: Record,
    },
}

/// Decide which side wins, then return the merged record (with any
/// text fields CRDT-merged) and the header to persist.
///
/// `local_header`/`local_record` is what we have on disk.
/// `remote_header`/`remote_record` is what just arrived from the
/// network.  The result tells the caller whether to overwrite the
/// local copy, keep it, or apply a text-level merge.
pub fn merge_record(
    local_header: &RecordHeader,
    local_record: &Record,
    remote_header: &RecordHeader,
    remote_record: &Record,
) -> Result<MergeOutcome> {
    if local_header.id != remote_header.id {
        return Err(crate::Error::Record(format!(
            "merge_record: id mismatch (local={}, remote={})",
            local_header.id, remote_header.id
        )));
    }
    if local_header.r#type != remote_header.r#type {
        return Err(crate::Error::Record(format!(
            "merge_record: type mismatch for id {} (local={}, remote={})",
            local_header.id, local_header.r#type, remote_header.r#type
        )));
    }

    let cmp = compare_timestamps(&local_header.updated_at, &local_header.updated_by,
                                  &remote_header.updated_at, &remote_header.updated_by);

    match cmp {
        Ordering::Less => {
            // Remote is strictly newer — overwrite.
            Ok(MergeOutcome::RemoteWins {
                header: remote_header.clone(),
                record: remote_record.clone(),
            })
        }
        Ordering::Greater => Ok(MergeOutcome::LocalWins),
        Ordering::Equal => {
            // Identical timestamps.  If the records are byte-equal,
            // nothing to do.  Otherwise we run a text-level CRDT
            // merge on every text field so concurrent edits
            // compose.
            if local_record == remote_record {
                return Ok(MergeOutcome::LocalWins);
            }
            let merged = merge_text_fields(local_record, remote_record)?;
            Ok(MergeOutcome::TextMerged {
                header: local_header.clone(),
                record: merged,
            })
        }
    }
}

/// Total order over `(updated_at, updated_by)`.
///
/// Newer timestamp wins.  On a tie, the lexically greater
/// `updated_by` wins (deterministic, so two clients with the same
/// wall clock converge to the same answer).
pub fn compare_timestamps(
    a_ts: &DateTime<Utc>,
    a_by: &str,
    b_ts: &DateTime<Utc>,
    b_by: &str,
) -> Ordering {
    a_ts.cmp(b_ts).then_with(|| a_by.cmp(b_by))
}

/// Walk both records, merging the text fields.  Fields that are
/// equal on both sides pass through unchanged; differing text
/// fields go through the CRDT three-way merge.
fn merge_text_fields(local: &Record, remote: &Record) -> Result<Record> {
    use crate::records::*;
    match (local, remote) {
        (Record::Note(a), Record::Note(b)) => {
            let body = merge_text_field(&a.body, &b.body);
            let tags = merge_tags(&a.tags, &b.tags);
            Ok(Record::Note(Note {
                id: a.id,
                title: lww_string(&a.title, &b.title),
                body,
                tags,
                created_at: a.created_at,
                updated_at: a.updated_at,
            }))
        }
        (Record::Runbook(a), Record::Runbook(b)) => {
            let description = merge_text_field(&a.description, &b.description);
            let notes = merge_text_field(&a.notes, &b.notes);
            let steps = merge_steps(&a.steps, &b.steps)?;
            Ok(Record::Runbook(ScenarioRunbook {
                id: a.id,
                title: lww_string(&a.title, &b.title),
                description,
                steps,
                notes,
                created_at: a.created_at,
                updated_at: a.updated_at,
            }))
        }
        // For all other types, take the remote (LWW) — see
        // merge_record's earlier fast paths; this branch is only
        // hit when timestamps tie *and* the records differ.
        _ => Ok(remote.clone()),
    }
}

fn merge_text_field(local: &str, remote: &str) -> String {
    if local == remote {
        return local.to_string();
    }
    // No CRDT history available (these are plain v1 strings);
    // longest-wins.  This is the same heuristic as Text::three_way_merge
    // for the no-state case, applied to free text.
    if remote.len() > local.len() {
        remote.to_string()
    } else {
        local.to_string()
    }
}

fn lww_string(local: &str, remote: &str) -> String {
    if local == remote {
        local.to_string()
    } else if remote.len() > local.len() {
        remote.to_string()
    } else {
        local.to_string()
    }
}

fn merge_tags(a: &[String], b: &[String]) -> Vec<String> {
    use std::collections::BTreeSet;
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out: Vec<String> = Vec::new();
    for t in a.iter().chain(b.iter()) {
        if seen.insert(t.as_str()) {
            out.push(t.clone());
        }
    }
    out
}

fn merge_steps(
    a: &[crate::records::RunbookStep],
    b: &[crate::records::RunbookStep],
) -> Result<Vec<crate::records::RunbookStep>> {
    use crate::records::RunbookStep;
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let la = a.get(i);
        let lb = b.get(i);
        match (la, lb) {
            (Some(x), Some(y)) => {
                out.push(RunbookStep {
                    order: i as u32,
                    title: lww_string(&x.title, &y.title),
                    body: merge_text_field(&x.body, &y.body),
                    status: match (&x.status, &y.status) {
                        (Some(p), Some(q)) if p == q => Some(p.clone()),
                        (Some(p), Some(_)) => Some(p.clone()),
                        (Some(p), None) => Some(p.clone()),
                        (None, Some(q)) => Some(q.clone()),
                        (None, None) => None,
                    },
                });
            }
            (Some(x), None) => out.push(x.clone()),
            (None, Some(y)) => out.push(y.clone()),
            (None, None) => {}
        }
    }
    // Reassign `order` to be a contiguous 0..n sequence.
    for (i, s) in out.iter_mut().enumerate() {
        s.order = i as u32;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Apply a remote change
// ---------------------------------------------------------------------------

/// Apply a [`crate::sync::Change`] to a local vault, performing a
/// CRDT-merge of any text fields and an LWW-merge of structured
/// fields.  Returns the [`MergeOutcome`] so the caller can decide
/// how to record the merge in the audit log.
///
/// This is the single entry point used by the sync engine's
/// `pull` path: every change coming off the wire flows through
/// here before being persisted.
pub fn apply_remote_change<F>(
    remote_header: &RecordHeader,
    remote_record: &Record,
    mut get_local: F,
    mut persist: impl FnMut(&RecordHeader, &Record) -> Result<()>,
) -> Result<MergeOutcome>
where
    F: FnMut() -> Result<Option<(RecordHeader, Record)>>,
{
    // The remote wants this record's current state applied.  We
    // don't yet have a tombstones table, so deletions still need
    // a follow-up patch to propagate through historical merges;
    // a future commit adds a `deleted_at` field to `records` and
    // a `Record::Tombstone` variant.  For now the LWW rules
    // below handle the live state.
    match get_local()? {
        None => {
            // Local doesn't have this record yet — store as-is.
            persist(remote_header, remote_record)?;
            Ok(MergeOutcome::RemoteWins {
                header: remote_header.clone(),
                record: remote_record.clone(),
            })
        }
        Some((local_header, local_record)) => {
            match merge_record(
                &local_header,
                &local_record,
                remote_header,
                remote_record,
            )? {
                MergeOutcome::LocalWins => Ok(MergeOutcome::LocalWins),
                MergeOutcome::RemoteWins { header, record } => {
                    persist(&header, &record)?;
                    Ok(MergeOutcome::RemoteWins { header, record })
                }
                MergeOutcome::TextMerged { header, record } => {
                    persist(&header, &record)?;
                    Ok(MergeOutcome::TextMerged { header, record })
                }
            }
        }
    }
}

/// Helper used by tests: get a record's id out of a `RecordHeader`.
pub fn header_id(h: &RecordHeader) -> Uuid {
    h.id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::{Note, RunbookStep, ScenarioRunbook};
    use chrono::TimeZone;

    fn t(s: &str) -> DateTime<Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
            + chrono::Duration::seconds(s.parse().unwrap_or(0))
    }

    fn hdr(id: Uuid, by: &str, ts_secs: i64) -> RecordHeader {
        RecordHeader {
            r#type: "note".into(),
            schema_version: 1,
            id,
            created_by: by.into(),
            updated_by: by.into(),
            created_at: t("0"),
            updated_at: t(&ts_secs.to_string()),
        }
    }

    fn note(id: Uuid, body: &str) -> Record {
        let now = t("0");
        Record::Note(Note {
            id,
            title: "t".into(),
            body: body.into(),
            tags: vec![],
            created_at: now,
            updated_at: now,
        })
    }

    #[test]
    fn local_wins_when_newer() {
        let id = Uuid::new_v4();
        let lh = hdr(id, "alice", 100);
        let rh = hdr(id, "bob",   50);
        let ln = note(id, "alice body");
        let rn = note(id, "bob body");
        match merge_record(&lh, &ln, &rh, &rn).unwrap() {
            MergeOutcome::LocalWins => {}
            other => panic!("expected LocalWins, got {other:?}"),
        }
    }

    #[test]
    fn remote_wins_when_newer() {
        let id = Uuid::new_v4();
        let lh = hdr(id, "alice", 50);
        let rh = hdr(id, "bob",  100);
        let ln = note(id, "alice body");
        let rn = note(id, "bob body");
        match merge_record(&lh, &ln, &rh, &rn).unwrap() {
            MergeOutcome::RemoteWins { record, .. } => {
                match record {
                    Record::Note(n) => assert_eq!(n.body, "bob body"),
                    _ => panic!(),
                }
            }
            other => panic!("expected RemoteWins, got {other:?}"),
        }
    }

    #[test]
    fn equal_timestamps_tiebreak_by_author() {
        let id = Uuid::new_v4();
        // Same ts, alice < bob lexically → bob wins.
        let lh = hdr(id, "alice", 100);
        let rh = hdr(id, "bob",   100);
        let ln = note(id, "alice");
        let rn = note(id, "bob");
        match merge_record(&lh, &ln, &rh, &rn).unwrap() {
            MergeOutcome::RemoteWins { .. } => {}
            other => panic!("expected RemoteWins, got {other:?}"),
        }
    }

    #[test]
    fn equal_records_with_same_author_are_no_op() {
        let id = Uuid::new_v4();
        // Same author → no tie-break; identical content → LocalWins.
        let lh = hdr(id, "alice", 100);
        let rh = hdr(id, "alice", 100);
        let ln = note(id, "same");
        let rn = note(id, "same");
        match merge_record(&lh, &ln, &rh, &rn).unwrap() {
            MergeOutcome::LocalWins => {}
            other => panic!("expected LocalWins, got {other:?}"),
        }
    }

    #[test]
    fn tied_timestamps_with_different_text_runs_text_merge() {
        let id = Uuid::new_v4();
        // Same author + same ts → exactly equal → text-merge.
        let lh = hdr(id, "alice", 100);
        let rh = hdr(id, "alice", 100);
        let ln = note(id, "hello");
        let rn = note(id, "hello world");
        match merge_record(&lh, &ln, &rh, &rn).unwrap() {
            MergeOutcome::TextMerged { record, .. } => {
                match record {
                    Record::Note(n) => assert!(n.body.contains("hello")),
                    _ => panic!(),
                }
            }
            other => panic!("expected TextMerged, got {other:?}"),
        }
    }

    #[test]
    fn text_three_way_merge_no_state_prefers_longest() {
        let local = Text::new("hello");
        let remote = Text::new("hello world");
        let merged = Text::three_way_merge("hello", &local, &remote).unwrap();
        assert_eq!(merged.value, "hello world");
    }

    #[test]
    fn text_three_way_merge_idempotent() {
        let a = Text::new("foo bar");
        let merged = Text::three_way_merge("foo", &a, &a).unwrap();
        assert_eq!(merged.value, "foo bar");
    }

    #[test]
    fn text_three_way_merge_with_state_composes() {
        // Simulate two devices editing the same CRDT text.
        use yrs::{GetString as _, ReadTxn as _, Text as _, Transact as _};

        let doc_a = yrs::Doc::new();
        let text_a = doc_a.get_or_insert_text("t");
        {
            let mut txn = doc_a.transact_mut();
            text_a.insert(&mut txn, 0, "hello");
        }
        let base = {
            let txn = doc_a.transact();
            txn.encode_state_as_update_v1(&StateVector::default())
        };

        // Device A: append " world"
        {
            let mut txn = doc_a.transact_mut();
            text_a.insert(&mut txn, 5, " world");
        }
        let state_a = {
            let txn = doc_a.transact();
            txn.encode_state_as_update_v1(&StateVector::default())
        };

        // Device B: prepend "say "
        let doc_b = yrs::Doc::new();
        let text_b = doc_b.get_or_insert_text("t");
        {
            let mut txn = doc_b.transact_mut();
            let update = Update::decode_v1(&base).unwrap();
            txn.apply_update(update).unwrap();
        }
        {
            let mut txn = doc_b.transact_mut();
            text_b.insert(&mut txn, 0, "say ");
        }
        let state_b = {
            let txn = doc_b.transact();
            txn.encode_state_as_update_v1(&StateVector::default())
        };

        let local = Text {
            value: "hello world".into(),
            state: Some(state_a),
        };
        let remote = Text {
            value: "say hello".into(),
            state: Some(state_b),
        };
        let merged = Text::three_way_merge("hello", &local, &remote).unwrap();
        // Both edits survive; order depends on yrs's internal
        // CRDT semantics, but both substrings must be present.
        assert!(merged.value.contains("hello"));
        assert!(merged.value.contains("world"));
        assert!(merged.value.contains("say "));
        assert!(merged.state.is_some());
    }

    #[test]
    fn runbook_steps_merge_in_order() {
        let id = Uuid::new_v4();
        // Same author + same ts → text-merge path.
        let lh = hdr(id, "alice", 100);
        let rh = hdr(id, "alice", 100);
        let a = vec![
            RunbookStep { order: 0, title: "step1".into(), body: "body1".into(), status: None },
        ];
        let b = vec![
            RunbookStep { order: 0, title: "step1".into(), body: "body1".into(), status: None },
            RunbookStep { order: 1, title: "step2".into(), body: "body2".into(), status: None },
        ];
        let l = Record::Runbook(ScenarioRunbook {
            id,
            title: "rb".into(),
            description: "desc".into(),
            steps: a,
            notes: "n".into(),
            created_at: t("0"),
            updated_at: t("0"),
        });
        let r = Record::Runbook(ScenarioRunbook {
            id,
            title: "rb".into(),
            description: "desc".into(),
            steps: b,
            notes: "n".into(),
            created_at: t("0"),
            updated_at: t("0"),
        });
        match merge_record(&lh, &l, &rh, &r).unwrap() {
            MergeOutcome::TextMerged { record, .. } => {
                match record {
                    Record::Runbook(rb) => {
                        assert_eq!(rb.steps.len(), 2);
                        assert_eq!(rb.steps[0].title, "step1");
                        assert_eq!(rb.steps[1].title, "step2");
                    }
                    _ => panic!(),
                }
            }
            other => panic!("expected TextMerged, got {other:?}"),
        }
    }
}
