//! CRDT sync engine: change feed, vector clocks, push/pull.
//!
//! In v1 the wire types and engine are wired up against opaque
//! blobs (the contents of the `records` table, plus the audit
//! tail).  The Y-CRDT integration is added in a follow-up commit;
//! the public API is stable so the Y-CRDT implementation can
//! drop in without breaking callers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single change.  Opaque to the server; only the actor pubkey
/// is meaningful to the protocol layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Change {
    /// Stable change id.
    pub id: Uuid,
    /// Lamport timestamp of the originating client.
    pub lamport: u64,
    /// Wall-clock timestamp of the originating client.
    pub ts: DateTime<Utc>,
    /// Username that authored this change.
    pub author: String,
    /// Affected record id (if any).
    pub record_id: Option<Uuid>,
    /// Opaque payload (e.g. an AEAD-wrapped record update).
    pub payload: Vec<u8>,
}

/// A vector clock: maps actor → counter.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct VectorClock {
    /// Inner map of actor to counter.
    pub counters: std::collections::BTreeMap<String, u64>,
}

impl VectorClock {
    /// Construct an empty clock.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bump the counter for `actor`.
    pub fn bump(&mut self, actor: &str) {
        *self.counters.entry(actor.to_string()).or_insert(0) += 1;
    }

    /// Merge another clock into this one, taking the max of each
    /// counter.  Returns true if this clock was advanced.
    pub fn merge(&mut self, other: &Self) -> bool {
        let mut changed = false;
        for (k, v) in &other.counters {
            let entry = self.counters.entry(k.clone()).or_insert(0);
            if *v > *entry {
                *entry = *v;
                changed = true;
            }
        }
        changed
    }

    /// Dominates: every counter in `self` is ≥ the corresponding
    /// counter in `other`.
    pub fn dominates(&self, other: &Self) -> bool {
        for (k, v) in &other.counters {
            let mine = self.counters.get(k).copied().unwrap_or(0);
            if mine < *v {
                return false;
            }
        }
        true
    }
}

pub mod protocol;
pub mod engine;
pub mod change;
pub mod update;
pub mod client;

pub use engine::Engine;
pub use protocol::{Request, Response};
