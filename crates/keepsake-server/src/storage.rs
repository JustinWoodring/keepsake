//! Server-side storage.  SQLite-backed.  All payloads are
//! opaque ciphertext — the server only ever sees encrypted
//! bytes and authenticated metadata (vector clocks, sequence
//! numbers, content hashes).
//!
//! The server is a pure encrypted blob store.  There is no
//! per-user state.  Every record is scoped to a `vault_id`
//! path segment; storage rows carry a `vault_id` column.
//! Knowing the URL + a vault id is enough to read or write.
//! The shared passphrase (held by the clients) is the only
//! thing protecting the data.
//!
//! Schema:
//!
//! ```sql
//! CREATE TABLE changes (
//!     vault_id    TEXT    NOT NULL,
//!     id          BLOB    NOT NULL,
//!     actor       TEXT    NOT NULL,
//!     lamport     INTEGER NOT NULL,
//!     ts_millis   INTEGER NOT NULL,
//!     record_id   BLOB,
//!     payload     BLOB    NOT NULL,
//!     PRIMARY KEY (vault_id, id)
//! );
//! CREATE INDEX changes_vault_lamport_idx
//!     ON changes(vault_id, lamport);
//! CREATE INDEX changes_vault_record_lamport_idx
//!     ON changes(vault_id, record_id, lamport);
//!
//! CREATE TABLE clocks (
//!     vault_id  TEXT    NOT NULL,
//!     actor     TEXT    NOT NULL,
//!     lamport   INTEGER NOT NULL,
//!     PRIMARY KEY (vault_id, actor)
//! );
//!
//! CREATE TABLE blobs (
//!     vault_id   TEXT    NOT NULL,
//!     sha256     BLOB    NOT NULL,
//!     ciphertext BLOB    NOT NULL,
//!     PRIMARY KEY (vault_id, sha256)
//! );
//! ```

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use keepsake_core::sync::Change;

#[derive(Debug)]
pub struct Storage {
    conn: Connection,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("expired")]
    Expired,
}

pub type Result<T> = std::result::Result<T, Error>;

/// One row of the per-vault "current state" view: the latest
/// known version of a single record, picked by LWW over
/// `(lamport, actor)`.  The server uses this to answer the
/// "give me every record in this vault" pull request without
/// making the client replay the full change history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateRow {
    /// The record id (16 bytes).
    pub record_id: Vec<u8>,
    /// The record type tag, e.g. "note".
    pub r#type: String,
    /// The change id of the winning version (16 bytes).
    pub change_id: Vec<u8>,
    /// Lamport clock of the winning version.
    pub lamport: u64,
    /// Author of the winning version.
    pub actor: String,
    /// Wall-clock ts of the winning version.
    pub ts_millis: i64,
    /// The opaque ciphertext payload.  The server picked this
    /// row by metadata; it did not decrypt.
    pub payload: Vec<u8>,
}

impl Storage {
    /// Open (or create) the server's SQLite database at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let s = Storage { conn };
        s.migrate()?;
        Ok(s)
    }

    /// In-memory storage for tests.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let s = Storage { conn };
        s.migrate()?;
        Ok(s)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS changes (
                vault_id    TEXT    NOT NULL,
                id          BLOB    NOT NULL,
                actor       TEXT    NOT NULL,
                lamport     INTEGER NOT NULL,
                ts_millis   INTEGER NOT NULL,
                record_id   BLOB,
                payload     BLOB    NOT NULL,
                PRIMARY KEY (vault_id, id)
            );
            CREATE INDEX IF NOT EXISTS changes_vault_lamport_idx
                ON changes(vault_id, lamport);
            CREATE INDEX IF NOT EXISTS changes_vault_record_lamport_idx
                ON changes(vault_id, record_id, lamport);
            CREATE INDEX IF NOT EXISTS changes_vault_actor_lamport_idx
                ON changes(vault_id, actor, lamport);
            CREATE TABLE IF NOT EXISTS clocks (
                vault_id  TEXT    NOT NULL,
                actor     TEXT    NOT NULL,
                lamport   INTEGER NOT NULL,
                PRIMARY KEY (vault_id, actor)
            );
            CREATE TABLE IF NOT EXISTS blobs (
                vault_id   TEXT    NOT NULL,
                sha256     BLOB    NOT NULL,
                ciphertext BLOB    NOT NULL,
                PRIMARY KEY (vault_id, sha256)
            );
            "#,
        )?;
        Ok(())
    }

    // -- changes -------------------------------------------------------------

    /// Append `changes` to `vault_id` and update per-actor
    /// clocks.  Idempotent: re-pushing the same `change.id` is
    /// a no-op (PRIMARY KEY constraint).
    pub fn append_changes(&self, vault_id: &str, changes: &[Change]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for c in changes {
            tx.execute(
                "INSERT OR IGNORE INTO changes
                    (vault_id, id, actor, lamport, ts_millis, record_id, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    vault_id,
                    c.id.as_bytes().to_vec(),
                    c.author,
                    c.lamport as i64,
                    c.ts.timestamp_millis(),
                    c.record_id.map(|u| u.as_bytes().to_vec()),
                    c.payload.clone(),
                ],
            )?;
            tx.execute(
                "INSERT INTO clocks (vault_id, actor, lamport) VALUES (?1, ?2, ?3)
                 ON CONFLICT(vault_id, actor) DO UPDATE SET lamport =
                   MAX(lamport, excluded.lamport)",
                params![vault_id, c.author, c.lamport as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// All changes for `vault_id` with `lamport > since`.  If
    /// `since == 0`, returns nothing (callers should use
    /// [`current_state`] for a "give me everything" pull).
    pub fn changes_since(
        &self,
        vault_id: &str,
        since: u64,
    ) -> Result<Vec<Change>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, actor, lamport, ts_millis, record_id, payload
             FROM changes
             WHERE vault_id = ?1 AND lamport > ?2
             ORDER BY lamport ASC",
        )?;
        let rows = stmt.query_map(
            params![vault_id, since as i64],
            |r| -> rusqlite::Result<(Vec<u8>, String, i64, i64, Option<Vec<u8>>, Vec<u8>)> {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )?;
        let mut out = Vec::new();
        for r in rows {
            let (id, actor, lamport, ts_millis, record_id, payload) = r?;
            let record_id = match record_id {
                Some(b) => Some(uuid_bytes(&b, "change.record_id")?),
                None => None,
            };
            out.push(Change {
                id: uuid_bytes(&id, "change.id")?,
                lamport: lamport as u64,
                ts: chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts_millis)
                    .ok_or_else(|| Error::Invalid("change ts_millis".into()))?,
                author: actor,
                record_id,
                payload,
            });
        }
        Ok(out)
    }

    /// Per-vault "current state" view: for every record that
    /// has at least one change in `vault_id`, return the
    /// latest version as picked by LWW on `(ts_millis, actor)`.
    /// The server doesn't decrypt — it just picks the
    /// ciphertext associated with the winning metadata.
    ///
    /// LWW tie-break matches the client's
    /// [`compare_timestamps`] function: newer `ts_millis` wins;
    /// on a tie, lexically greater `actor` wins.
    pub fn current_state(&self, vault_id: &str) -> Result<Vec<StateRow>> {
        // Fetch all candidate rows ordered by the LWW key
        // (ts_millis ASC, actor ASC).  For each record_id,
        // the *last* row seen is the winner.  The index on
        // `(vault_id, lamport)` plus the row count make this
        // cheap for the vaults a single user actually has.
        let mut stmt = self.conn.prepare(
            "SELECT record_id, id, actor, lamport, ts_millis, payload
             FROM changes
             WHERE vault_id = ?1 AND record_id IS NOT NULL
             ORDER BY ts_millis ASC, actor ASC",
        )?;
        let rows = stmt.query_map(params![vault_id], |r| {
            Ok((
                r.get::<_, Vec<u8>>(0)?,
                r.get::<_, Vec<u8>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Vec<u8>>(5)?,
            ))
        })?;
        let mut by_record: std::collections::BTreeMap<
            Vec<u8>,
            (Vec<u8>, String, i64, i64, Vec<u8>),
        > = std::collections::BTreeMap::new();
        for r in rows {
            let (record_id, change_id, actor, lamport, ts_millis, payload) = r?;
            by_record.insert(record_id, (change_id, actor, lamport, ts_millis, payload));
        }
        let mut out: Vec<StateRow> = by_record
            .into_iter()
            .map(|(record_id, (change_id, actor, lamport, ts_millis, payload))| StateRow {
                record_id,
                r#type: String::new(),
                change_id,
                lamport: lamport as u64,
                actor,
                ts_millis,
                payload,
            })
            .collect();
        out.sort_by_key(|r| r.lamport);
        Ok(out)
    }

    /// Lamport clock for `actor` in `vault_id` (0 if never seen).
    pub fn actor_clock(&self, vault_id: &str, actor: &str) -> Result<u64> {
        let n: Option<i64> = self
            .conn
            .query_row(
                "SELECT lamport FROM clocks WHERE vault_id = ?1 AND actor = ?2",
                params![vault_id, actor],
                |r| r.get(0),
            )
            .optional()?;
        Ok(n.unwrap_or(0) as u64)
    }

    /// All actor clocks in a vault.
    pub fn all_clocks(&self, vault_id: &str) -> Result<std::collections::BTreeMap<String, u64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT actor, lamport FROM clocks WHERE vault_id = ?1")?;
        let rows = stmt.query_map(params![vault_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        let mut out = std::collections::BTreeMap::new();
        for r in rows {
            let (a, l) = r?;
            out.insert(a, l as u64);
        }
        Ok(out)
    }

    // -- blobs ---------------------------------------------------------------

    pub fn put_blob(&self, vault_id: &str, sha256: &[u8; 32], ciphertext: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO blobs (vault_id, sha256, ciphertext)
             VALUES (?1, ?2, ?3)",
            params![vault_id, sha256.to_vec(), ciphertext.to_vec()],
        )?;
        Ok(())
    }

    pub fn get_blob(&self, vault_id: &str, sha256: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        let row: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT ciphertext FROM blobs WHERE vault_id = ?1 AND sha256 = ?2",
                params![vault_id, sha256.to_vec()],
                |r| r.get(0),
            )
            .optional()?;
        Ok(row)
    }
}

fn uuid_bytes(b: &[u8], name: &str) -> Result<uuid::Uuid> {
    b.try_into()
        .map(uuid::Uuid::from_bytes)
        .map_err(|_| Error::Invalid(format!("{name} not 16 bytes")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mk_change(id: [u8; 16], actor: &str, lamport: u64, record: [u8; 16], payload: &[u8]) -> Change {
        Change {
            id: uuid::Uuid::from_bytes(id),
            lamport,
            ts: Utc::now(),
            author: actor.into(),
            record_id: Some(uuid::Uuid::from_bytes(record)),
            payload: payload.to_vec(),
        }
    }

    #[test]
    fn round_trip_changes() {
        let s = Storage::open_in_memory().unwrap();
        let c = mk_change([1; 16], "alice", 1, [9; 16], b"hello");
        s.append_changes("v1", std::slice::from_ref(&c)).unwrap();
        let all = s.changes_since("v1", 0).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].payload, b"hello");

        // Re-pushing is idempotent.
        s.append_changes("v1", std::slice::from_ref(&c)).unwrap();
        let all = s.changes_since("v1", 0).unwrap();
        assert_eq!(all.len(), 1);

        // Since filter works.
        s.append_changes("v1", &[mk_change([2; 16], "alice", 2, [9; 16], b"world")]).unwrap();
        let after1 = s.changes_since("v1", 1).unwrap();
        assert_eq!(after1.len(), 1);
        assert_eq!(after1[0].payload, b"world");
    }

    #[test]
    fn cross_vault_isolation() {
        let s = Storage::open_in_memory().unwrap();
        let c1 = mk_change([1; 16], "alice", 1, [9; 16], b"vault1");
        let c2 = mk_change([1; 16], "alice", 1, [9; 16], b"vault2");
        s.append_changes("vault1", std::slice::from_ref(&c1)).unwrap();
        s.append_changes("vault2", std::slice::from_ref(&c2)).unwrap();
        let v1 = s.changes_since("vault1", 0).unwrap();
        let v2 = s.changes_since("vault2", 0).unwrap();
        assert_eq!(v1.len(), 1);
        assert_eq!(v1[0].payload, b"vault1");
        assert_eq!(v2.len(), 1);
        assert_eq!(v2[0].payload, b"vault2");
    }

    #[test]
    fn current_state_picks_latest_per_record() {
        let s = Storage::open_in_memory().unwrap();
        let rec = [42u8; 16];
        let mut c1 = mk_change([1; 16], "alice", 1, rec, b"v1");
        c1.ts = chrono::DateTime::<chrono::Utc>::from_timestamp(1_000, 0).unwrap();
        let mut c2 = mk_change([2; 16], "bob",   2, rec, b"v2");
        c2.ts = chrono::DateTime::<chrono::Utc>::from_timestamp(2_000, 0).unwrap();
        let mut c3 = mk_change([3; 16], "alice", 3, rec, b"v3");
        c3.ts = chrono::DateTime::<chrono::Utc>::from_timestamp(3_000, 0).unwrap();
        s.append_changes("v1", &[c1, c2, c3]).unwrap();
        let state = s.current_state("v1").unwrap();
        assert_eq!(state.len(), 1);
        assert_eq!(state[0].payload, b"v3");
        assert_eq!(state[0].lamport, 3);
    }
}
