//! Server-side storage.  SQLite-backed.  All payloads are
//! opaque ciphertext — the server only ever sees encrypted
//! bytes and authenticated metadata (vector clocks, sequence
//! numbers, content hashes).
//!
//! Schema:
//!
//! ```sql
//! CREATE TABLE users (
//!     username    TEXT PRIMARY KEY,
//!     envelope_pk BLOB NOT NULL,        -- 32 bytes
//!     created_at  INTEGER NOT NULL      -- Unix seconds
//! );
//!
//! CREATE TABLE challenges (
//!     challenge  BLOB PRIMARY KEY,      -- 32 random bytes
//!     username   TEXT NOT NULL,
//!     expires_at INTEGER NOT NULL       -- Unix seconds
//! );
//!
//! CREATE TABLE sessions (
//!     bearer     BLOB PRIMARY KEY,      -- 32 random bytes
//!     username   TEXT NOT NULL,
//!     expires_at INTEGER NOT NULL       -- Unix seconds
//! );
//!
//! CREATE TABLE changes (
//!     id          BLOB PRIMARY KEY,     -- 16-byte change UUID
//!     actor       TEXT NOT NULL,
//!     lamport     INTEGER NOT NULL,
//!     ts_millis   INTEGER NOT NULL,
//!     record_id   BLOB,                 -- nullable 16-byte UUID
//!     payload     BLOB NOT NULL
//! );
//! CREATE INDEX changes_actor_lamport_idx ON changes(actor, lamport);
//!
//! CREATE TABLE clocks (
//!     actor    TEXT PRIMARY KEY,
//!     lamport  INTEGER NOT NULL
//! );
//!
//! CREATE TABLE blobs (
//!     sha256    BLOB PRIMARY KEY,       -- 32 bytes
//!     ciphertext BLOB NOT NULL
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

/// A persisted user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRow {
    pub username: String,
    pub envelope_pk: [u8; 32],
    pub created_at: i64,
}

/// A persisted challenge.
#[derive(Debug, Clone)]
pub struct ChallengeRow {
    pub challenge: [u8; 32],
    pub username: String,
    pub expires_at: i64,
}

/// A persisted session.
#[derive(Debug, Clone)]
pub struct SessionRow {
    pub bearer: [u8; 32],
    pub username: String,
    pub expires_at: i64,
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
            CREATE TABLE IF NOT EXISTS users (
                username    TEXT PRIMARY KEY,
                envelope_pk BLOB    NOT NULL,
                created_at  INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS challenges (
                challenge  BLOB    PRIMARY KEY,
                username   TEXT    NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS sessions (
                bearer     BLOB    PRIMARY KEY,
                username   TEXT    NOT NULL,
                expires_at INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS changes (
                id          BLOB    PRIMARY KEY,
                actor       TEXT    NOT NULL,
                lamport     INTEGER NOT NULL,
                ts_millis   INTEGER NOT NULL,
                record_id   BLOB,
                payload     BLOB    NOT NULL
            );
            CREATE INDEX IF NOT EXISTS changes_actor_lamport_idx
                ON changes(actor, lamport);
            CREATE TABLE IF NOT EXISTS clocks (
                actor    TEXT    PRIMARY KEY,
                lamport  INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS blobs (
                sha256     BLOB    PRIMARY KEY,
                ciphertext BLOB    NOT NULL
            );
            "#,
        )?;
        Ok(())
    }

    // -- users ---------------------------------------------------------------

    pub fn put_user(&self, row: &UserRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO users (username, envelope_pk, created_at)
             VALUES (?1, ?2, ?3)",
            params![row.username, row.envelope_pk.to_vec(), row.created_at],
        )?;
        Ok(())
    }

    pub fn get_user(&self, username: &str) -> Result<Option<UserRow>> {
        let row: Option<(String, Vec<u8>, i64)> = self
            .conn
            .query_row(
                "SELECT username, envelope_pk, created_at FROM users WHERE username = ?1",
                params![username],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some((username, pk, created_at)) => {
                let pk: [u8; 32] = pk.try_into().map_err(|_| {
                    Error::Invalid(format!("envelope_pk wrong size for {username}"))
                })?;
                Ok(Some(UserRow { username, envelope_pk: pk, created_at }))
            }
        }
    }

    // -- challenges ----------------------------------------------------------

    pub fn put_challenge(&self, row: &ChallengeRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO challenges (challenge, username, expires_at)
             VALUES (?1, ?2, ?3)",
            params![row.challenge.to_vec(), row.username, row.expires_at],
        )?;
        Ok(())
    }

    pub fn take_challenge(&self, challenge: &[u8; 32]) -> Result<ChallengeRow> {
        // Atomic: delete and return in one transaction.
        let mut stmt = self.conn.prepare(
            "SELECT username, expires_at FROM challenges WHERE challenge = ?1",
        )?;
        let row: Option<(String, i64)> = stmt
            .query_row(params![challenge.to_vec()], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        let (username, expires_at) = row.ok_or(Error::NotFound("challenge".into()))?;
        self.conn.execute(
            "DELETE FROM challenges WHERE challenge = ?1",
            params![challenge.to_vec()],
        )?;
        Ok(ChallengeRow { challenge: *challenge, username, expires_at })
    }

    // -- sessions ------------------------------------------------------------

    pub fn put_session(&self, row: &SessionRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (bearer, username, expires_at)
             VALUES (?1, ?2, ?3)",
            params![row.bearer.to_vec(), row.username, row.expires_at],
        )?;
        Ok(())
    }

    pub fn get_session(&self, bearer: &[u8; 32]) -> Result<Option<SessionRow>> {
        let row: Option<(String, i64)> = self
            .conn
            .query_row(
                "SELECT username, expires_at FROM sessions WHERE bearer = ?1",
                params![bearer.to_vec()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        match row {
            None => Ok(None),
            Some((username, expires_at)) => Ok(Some(SessionRow {
                bearer: *bearer,
                username,
                expires_at,
            })),
        }
    }

    pub fn delete_session(&self, bearer: &[u8; 32]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sessions WHERE bearer = ?1",
            params![bearer.to_vec()],
        )?;
        Ok(())
    }

    pub fn delete_expired_sessions(&self, now: i64) -> Result<usize> {
        let n = self.conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?1",
            params![now],
        )?;
        Ok(n)
    }

    // -- changes -------------------------------------------------------------

    /// Append `changes` and update the per-actor clock.  Returns
    /// the per-actor clock after the append for each touched
    /// actor.  Idempotent: re-pushing the same `change.id` is a
    /// no-op.
    pub fn append_changes(&self, changes: &[Change]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for c in changes {
            tx.execute(
                "INSERT OR IGNORE INTO changes
                    (id, actor, lamport, ts_millis, record_id, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    c.id.as_bytes().to_vec(),
                    c.author,
                    c.lamport as i64,
                    c.ts.timestamp_millis(),
                    c.record_id.map(|u| u.as_bytes().to_vec()),
                    c.payload.clone(),
                ],
            )?;
            // Bump the actor's clock to the max of the current
            // and the new lamport.
            tx.execute(
                "INSERT INTO clocks (actor, lamport) VALUES (?1, ?2)
                 ON CONFLICT(actor) DO UPDATE SET lamport =
                   MAX(lamport, excluded.lamport)",
                params![c.author, c.lamport as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// All changes for actor `actor` with `lamport > since`.  If
    /// `actor` is `None`, returns all changes for every actor
    /// with `lamport > since`.
    pub fn changes_since(
        &self,
        actor: Option<&str>,
        since: u64,
    ) -> Result<Vec<Change>> {
        let rows: Vec<(Vec<u8>, String, i64, i64, Option<Vec<u8>>, Vec<u8>)> = match actor {
            Some(a) => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, actor, lamport, ts_millis, record_id, payload
                     FROM changes WHERE actor = ?1 AND lamport > ?2
                     ORDER BY lamport ASC",
                )?;
                let rows = stmt
                    .query_map(params![a, since as i64], |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                        ))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                rows
            }
            None => {
                let mut stmt = self.conn.prepare(
                    "SELECT id, actor, lamport, ts_millis, record_id, payload
                     FROM changes WHERE lamport > ?1
                     ORDER BY lamport ASC",
                )?;
                let rows = stmt
                    .query_map(params![since as i64], |r| {
                        Ok((
                            r.get(0)?,
                            r.get(1)?,
                            r.get(2)?,
                            r.get(3)?,
                            r.get(4)?,
                            r.get(5)?,
                        ))
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                rows
            }
        };
        rows.into_iter()
            .map(|(id, actor, lamport, ts_millis, record_id, payload)| {
                let id = uuid_bytes(&id, "change.id")?;
                let record_id = match record_id {
                    Some(b) => Some(uuid_bytes(&b, "change.record_id")?),
                    None => None,
                };
                let ts = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ts_millis)
                    .ok_or_else(|| Error::Invalid("change ts_millis".into()))?;
                Ok(Change {
                    id,
                    lamport: lamport as u64,
                    ts,
                    author: actor,
                    record_id,
                    payload,
                })
            })
            .collect()
    }

    /// The current lamport clock for `actor` (0 if never seen).
    pub fn actor_clock(&self, actor: &str) -> Result<u64> {
        let n: Option<i64> = self
            .conn
            .query_row(
                "SELECT lamport FROM clocks WHERE actor = ?1",
                params![actor],
                |r| r.get(0),
            )
            .optional()?;
        Ok(n.unwrap_or(0) as u64)
    }

    /// All actor clocks (one row per actor seen by the server).
    pub fn all_clocks(&self) -> Result<std::collections::BTreeMap<String, u64>> {
        let mut stmt = self.conn.prepare("SELECT actor, lamport FROM clocks")?;
        let rows = stmt.query_map([], |r| {
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

    pub fn put_blob(&self, sha256: &[u8; 32], ciphertext: &[u8]) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO blobs (sha256, ciphertext) VALUES (?1, ?2)",
            params![sha256.to_vec(), ciphertext.to_vec()],
        )?;
        Ok(())
    }

    pub fn get_blob(&self, sha256: &[u8; 32]) -> Result<Option<Vec<u8>>> {
        let row: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT ciphertext FROM blobs WHERE sha256 = ?1",
                params![sha256.to_vec()],
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
