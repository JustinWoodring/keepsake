//! Append-only audit log.  Each entry is hash-chained: the
//! `prev_hash` of entry N+1 is the blake3 hash of entry N.  The
//! chain is verified on read; any tampering surfaces as an error
//! pointing at the first broken entry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// What kind of operation an audit entry records.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditOp {
    /// Vault unlocked.
    Unlock,
    /// Vault locked.
    Lock,
    /// Record created.
    Create,
    /// Record updated.
    Update,
    /// Record deleted.
    Delete,
    /// User added.
    AddUser,
    /// User removed.
    RemoveUser,
    /// Password rotated.
    RotatePassword,
    /// Sync push.
    SyncPush,
    /// Sync pull.
    SyncPull,
    /// Encrypted export.
    Export,
    /// Encrypted import.
    Import,
    /// Custom / free-form.
    Other,
}

/// A single audit entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEntry {
    /// Monotonically increasing sequence number.
    pub seq: u64,
    /// Operation kind.
    pub op: AuditOp,
    /// Acting username.
    pub actor: String,
    /// Affected record id (if any).
    pub target_id: Option<String>,
    /// Free-form details.
    pub details: Option<String>,
    /// Wall-clock timestamp.
    pub ts: DateTime<Utc>,
    /// blake3 hash of the previous entry's serialized form.  For
    /// the genesis entry this is the 32 zero bytes.
    pub prev_hash: [u8; 32],
    /// blake3 hash of this entry's serialized form.
    pub hash: [u8; 32],
}

/// Compute the entry hash.  Deterministic: same entry contents →
/// same hash.  Uses the timestamp truncated to whole seconds so
/// the hash is stable across read/write round-trips (sub-second
/// precision is not preserved in the on-disk schema).
pub fn entry_hash(
    seq: u64,
    op: AuditOp,
    actor: &str,
    target_id: Option<&str>,
    details: Option<&str>,
    ts: &DateTime<Utc>,
    prev_hash: &[u8; 32],
) -> [u8; 32] {
    let mut h = blake3::Hasher::new();
    h.update(&seq.to_le_bytes());
    h.update(&[op as u8]);
    h.update(actor.as_bytes());
    h.update(target_id.unwrap_or("").as_bytes());
    h.update(details.unwrap_or("").as_bytes());
    h.update(&ts.timestamp().to_le_bytes());
    h.update(prev_hash);
    *h.finalize().as_bytes()
}

/// Verify an entire audit chain.  Returns the seq of the first
/// broken entry, or `Ok(())` if the chain is intact.
pub fn verify_chain(entries: &[AuditEntry]) -> Result<()> {
    let mut prev = [0u8; 32];
    for e in entries {
        let expected = entry_hash(
            e.seq,
            e.op,
            &e.actor,
            e.target_id.as_deref(),
            e.details.as_deref(),
            &e.ts,
            &prev,
        );
        if expected != e.hash {
            return Err(Error::AuditTampered(e.seq));
        }
        if e.prev_hash != prev {
            return Err(Error::AuditTampered(e.seq));
        }
        prev = e.hash;
    }
    Ok(())
}

/// Find the index of the first entry that doesn't hash
/// correctly under the current `entry_hash` function, given
/// each entry's own `prev_hash`.
///
/// Returns the index of the first untrusted entry, or
/// `entries.len()` if every entry is trusted.  Note: this is
/// "untrusted" in the per-entry sense; the chain as a whole
/// can still be broken at a later entry if `prev_hash`
/// references a hash that doesn't match.
pub fn first_untrusted(entries: &[AuditEntry]) -> usize {
    for (i, e) in entries.iter().enumerate() {
        let expected = entry_hash(
            e.seq,
            e.op,
            &e.actor,
            e.target_id.as_deref(),
            e.details.as_deref(),
            &e.ts,
            &e.prev_hash,
        );
        if expected != e.hash {
            return i;
        }
    }
    entries.len()
}

/// Re-anchor an untrusted chain: drop every entry at or before
/// the first one that doesn't hash correctly under the current
/// function, and re-chain the survivors starting from a fresh
/// genesis with new seq numbers starting at 1.
///
/// The survivors' original `seq` numbers shift down to a
/// contiguous 1..N range.  All surviving fields are preserved
/// (op, actor, target_id, details, ts); only `seq`, `prev_hash`,
/// and `hash` are rewritten.
///
/// Returns the rewritten entries.  This function does not
/// touch the database — call [`crate::vault::Vault::rewrite_audit_chain`]
/// to persist the result.
pub fn rebuild_chain(entries: &[AuditEntry]) -> Vec<AuditEntry> {
    // `first_untrusted` returns the index of the first entry
    // that doesn't verify, or `entries.len()` if every entry
    // verifies.  We drop the untrusted prefix AND the first
    // untrusted entry itself, keeping the rest.  When every
    // entry is trusted, we keep them all.
    let cut = first_untrusted(entries);
    let survivors: &[AuditEntry] = if cut < entries.len() {
        // Drop `cut + 1` entries (the prefix up to and
        // including the first untrusted one).
        &entries[cut + 1..]
    } else {
        // All trusted; keep everything.
        entries
    };
    let mut out = Vec::with_capacity(survivors.len());
    let mut prev = [0u8; 32];
    for (i, e) in survivors.iter().enumerate() {
        let new_seq = (i + 1) as u64;
        let new_hash = entry_hash(
            new_seq,
            e.op,
            &e.actor,
            e.target_id.as_deref(),
            e.details.as_deref(),
            &e.ts,
            &prev,
        );
        out.push(AuditEntry {
            seq: new_seq,
            op: e.op,
            actor: e.actor.clone(),
            target_id: e.target_id.clone(),
            details: e.details.clone(),
            ts: e.ts,
            prev_hash: prev,
            hash: new_hash,
        });
        prev = new_hash;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 6, 17, 10, 0, 0).unwrap()
    }

    fn mk_entry(seq: u64, prev: [u8; 32], actor: &str) -> AuditEntry {
        let h = entry_hash(
            seq,
            AuditOp::Unlock,
            actor,
            None,
            None,
            &ts(),
            &prev,
        );
        AuditEntry {
            seq,
            op: AuditOp::Unlock,
            actor: actor.into(),
            target_id: None,
            details: None,
            ts: ts(),
            prev_hash: prev,
            hash: h,
        }
    }

    #[test]
    fn chain_verifies() {
        let a = mk_entry(1, [0u8; 32], "justin");
        let b = mk_entry(2, a.hash, "dahlia");
        let c = mk_entry(3, b.hash, "justin");
        verify_chain(&[a, b, c]).unwrap();
    }

    #[test]
    fn chain_detects_tamper() {
        let a = mk_entry(1, [0u8; 32], "justin");
        let b = mk_entry(2, a.hash, "dahlia");
        // Tamper with the actor field but keep the hash.
        let mut bad = b.clone();
        bad.actor = "mallory".into();
        let err = verify_chain(&[a, bad]).unwrap_err();
        match err {
            Error::AuditTampered(2) => {}
            _ => panic!("expected AuditTampered(2)"),
        }
    }

    #[test]
    fn first_untrusted_finds_bad_hash() {
        let a = mk_entry(1, [0u8; 32], "justin");
        let b = mk_entry(2, a.hash, "dahlia");
        // Tamper b's hash (simulate an entry written by a
        // different algorithm).  Its fields are still readable
        // but the stored hash doesn't match.
        let mut bad = b.clone();
        bad.hash = [0u8; 32];
        let entries = vec![a, bad];
        assert_eq!(first_untrusted(&entries), 1);
    }

    #[test]
    fn first_untrusted_zero_when_all_trusted() {
        let a = mk_entry(1, [0u8; 32], "justin");
        let b = mk_entry(2, a.hash, "dahlia");
        let c = mk_entry(3, b.hash, "justin");
        let entries = vec![a, b, c];
        assert_eq!(first_untrusted(&entries), 3);
    }

    #[test]
    fn rebuild_chain_drops_legacy_and_rehashes() {
        // Simulate: 1 untrusted legacy entry (genesis) + 3
        // trusted entries chained from a hash that doesn't
        // match the current function.
        let legacy = AuditEntry {
            seq: 1,
            op: AuditOp::AddUser,
            actor: "alice".into(),
            target_id: Some("alice".into()),
            details: Some("vault initialized".into()),
            ts: ts(),
            prev_hash: [0u8; 32],
            hash: [0xab; 32], // intentionally wrong
        };
        let a = mk_entry(2, [0u8; 32], "justin"); // hash will be valid under current fn
        let b = mk_entry(3, a.hash, "dahlia");
        let c = mk_entry(4, b.hash, "justin");
        let entries = vec![legacy, a, b, c];

        let rebuilt = rebuild_chain(&entries);
        assert_eq!(rebuilt.len(), 3);
        assert_eq!(rebuilt[0].seq, 1);
        assert_eq!(rebuilt[1].seq, 2);
        assert_eq!(rebuilt[2].seq, 3);
        // Genesis anchor.
        assert_eq!(rebuilt[0].prev_hash, [0u8; 32]);
        // Chain links.
        assert_eq!(rebuilt[1].prev_hash, rebuilt[0].hash);
        assert_eq!(rebuilt[2].prev_hash, rebuilt[1].hash);
        // Now verifies under the current function.
        verify_chain(&rebuilt).unwrap();
    }

    #[test]
    fn rebuild_chain_with_all_trusted_is_a_noop_on_content() {
        let a = mk_entry(1, [0u8; 32], "justin");
        let b = mk_entry(2, a.hash, "dahlia");
        let entries = vec![a, b];
        let rebuilt = rebuild_chain(&entries);
        assert_eq!(rebuilt.len(), 2);
        // Content preserved; only seq/hash are re-anchored.
        assert_eq!(rebuilt[0].actor, "justin");
        assert_eq!(rebuilt[1].actor, "dahlia");
        verify_chain(&rebuilt).unwrap();
    }
}
