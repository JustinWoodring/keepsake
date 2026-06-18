//! Apply a remote [`Change`] to a local vault.
//!
//! The wire-level `Change` struct is opaque; the engine only
//! knows the record id.  This module is where the payload gets
//! decoded back into a [`RecordHeader`] + [`Record`] and run
//! through the CRDT layer before being persisted.
//!
//! ## Wire payload format
//!
//! A `Change.payload` for a record update is the
//! `bincode`-serialized tuple `(RecordHeader, Record)`.  An
//! AEAD-encrypted payload is the next iteration; the AEAD
//! material comes from a per-record data key derived from the
//! vault key, so a future patch can seal the payload without
//! changing the protocol.
//!
//! `apply_change` deserializes the payload, calls
//! [`crate::crdt::apply_remote_change`] to do the merge, and
//! returns a [`ApplyOutcome`] describing what happened.

use serde::{Deserialize, Serialize};

use crate::crdt::{apply_remote_change, MergeOutcome};
use crate::records::{Record, RecordHeader};
use crate::vault::Vault;
use crate::Result;

/// What happened when we applied a remote change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Local copy was strictly newer.  No change persisted.
    LocalWins,
    /// Remote copy was strictly newer.  Local was overwritten.
    RemoteWins,
    /// Both sides had the same logical timestamp but different
    /// text.  We ran a text-level CRDT merge; the local copy is
    /// now the union of both edits.
    TextMerged,
    /// Local didn't have this record; it was created.
    Created,
}

/// Encoded form of a `Change.payload` for a record-level update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncodedRecord {
    /// Header of the record in the change.
    pub header: RecordHeader,
    /// Body of the record in the change.
    pub record: Record,
}

/// Decode the payload of a record-level change.  Returns `None`
/// for non-record changes (e.g. blob uploads) — those are
/// handled elsewhere.
pub fn decode(payload: &[u8]) -> Result<Option<EncodedRecord>> {
    match serde_json::from_slice::<EncodedRecord>(payload) {
        Ok(r) => Ok(Some(r)),
        Err(_) => Ok(None),
    }
}

/// Encode a header+record into a `Change.payload`.
pub fn encode(header: &RecordHeader, record: &Record) -> Result<Vec<u8>> {
    let enc = EncodedRecord { header: header.clone(), record: record.clone() };
    Ok(serde_json::to_vec(&enc)?)
}

/// Apply a remote change to a local vault.
///
/// Returns:
/// - `Ok(None)` for non-record changes (e.g. blob upload; out of
///   scope here).
/// - `Ok(Some(outcome))` for record-level changes.
pub fn apply_change(vault: &mut Vault, change: &crate::sync::Change) -> Result<Option<ApplyOutcome>> {
    let Some(encoded) = decode(&change.payload)? else {
        return Ok(None);
    };
    let remote_header = encoded.header;
    let remote_record = encoded.record;

    // Use a dummy username for the apply path — the actual
    // audit entry is appended by the sync engine.
    let local_existed = match vault.get_record(remote_header.id) {
        Ok(_) => true,
        Err(crate::Error::NotFound(_)) => false,
        Err(crate::Error::Storage(rusqlite::Error::QueryReturnedNoRows)) => false,
        Err(e) => return Err(e),
    };

    let outcome = apply_remote_change(
        &remote_header,
        &remote_record,
        || -> Result<Option<(RecordHeader, Record)>> {
            match vault.get_record(remote_header.id) {
                Ok((h, r)) => Ok(Some((h, r))),
                Err(crate::Error::NotFound(_)) => Ok(None),
                Err(crate::Error::Storage(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
                Err(e) => Err(e),
            }
        },
        |h, r| {
            vault.put_record(h, r)?;
            Ok(())
        },
    )?;

    let summary = match outcome {
        MergeOutcome::LocalWins if !local_existed => ApplyOutcome::Created,
        MergeOutcome::LocalWins  => ApplyOutcome::LocalWins,
        MergeOutcome::RemoteWins { .. } if !local_existed => ApplyOutcome::Created,
        MergeOutcome::RemoteWins { .. } => ApplyOutcome::RemoteWins,
        MergeOutcome::TextMerged { .. } => ApplyOutcome::TextMerged,
    };
    Ok(Some(summary))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::records::Note;
    use crate::crypto::aead::random_key;
    use crate::identity::VaultKey;
    use chrono::Utc;
    use uuid::Uuid;

    fn unlock(v: &mut Vault) {
        let key = VaultKey::from_bytes(*random_key().0);
        v.unlock(&key).unwrap();
    }

    #[test]
    fn round_trip_header_record_payload() {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let h = RecordHeader {
            r#type: "note".into(),
            schema_version: 1,
            id,
            created_by: "alice".into(),
            updated_by: "alice".into(),
            created_at: now,
            updated_at: now,
        };
        let r = Record::Note(Note {
            id,
            title: "t".into(),
            body: "b".into(),
            tags: vec![],
            created_at: now,
            updated_at: now,
        });
        let bytes = encode(&h, &r).unwrap();
        let dec = decode(&bytes).unwrap().unwrap();
        assert_eq!(dec.header.id, id);
        match dec.record {
            Record::Note(n) => assert_eq!(n.body, "b"),
            _ => panic!(),
        }
    }

    #[test]
    fn apply_change_stores_new_record() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = Vault::open_or_create(&dir.path().join("v.db")).unwrap();
        unlock(&mut v);

        let id = Uuid::new_v4();
        let now = Utc::now();
        let h = RecordHeader {
            r#type: "note".into(),
            schema_version: 1,
            id,
            created_by: "bob".into(),
            updated_by: "bob".into(),
            created_at: now,
            updated_at: now,
        };
        let r = Record::Note(Note {
            id,
            title: "remote".into(),
            body: "hello".into(),
            tags: vec![],
            created_at: now,
            updated_at: now,
        });
        let bytes = encode(&h, &r).unwrap();
        let ch = crate::sync::Change {
            id: Uuid::new_v4(),
            lamport: 1,
            ts: now,
            author: "bob".into(),
            record_id: Some(id),
            payload: bytes,
        };
        let outcome = apply_change(&mut v, &ch).unwrap();
        assert!(matches!(outcome, Some(ApplyOutcome::Created) | Some(ApplyOutcome::RemoteWins)));

        let (_h2, rec) = v.get_record(id).unwrap();
        match rec {
            Record::Note(n) => assert_eq!(n.body, "hello"),
            _ => panic!(),
        }
    }
}
