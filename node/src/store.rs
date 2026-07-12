//! Persistent local store for a single node (P1: one embedded SQLite DB per node).
//!
//! Local-first invariant (D3/D-local-first): no server, no network. Each node owns a
//! file-backed SQLite database (`rusqlite` + `bundled` SQLite, zero system deps) that is
//! the durable source of truth for:
//!   - node identity (eid, pk, hybrid pk, sk) — so a node can be reconstructed on restart,
//!   - the custody bundle queue (BIBE custody transfers accepted into store-and-forward),
//!   - the replay-dedup set (source, creation_ts).
//!
//! The in-memory [`crate::Node`] API is unchanged; this module is a durability layer that
//! is opted into via [`crate::Node::open_store`] and flushed via
//! [`crate::Node::save_state`]/[`crate::Node::load_state`]. Restart scenario:
//! `new()` is deterministic per seed, so `open_store` + `load_state` restores the exact
//! custody/seen state across process boundaries.

use crate::Bundle;
use dowiz_kernel::pq::envelope::SignedEnvelope;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// Errors surfaced by the durability layer.
#[derive(Debug)]
pub enum StoreError {
    /// Underlying SQLite I/O or schema error.
    Rusqlite(rusqlite::Error),
    /// (De)serialization of a persisted row failed — e.g. a corrupted payload blob.
    Serde(String),
    /// The identity row on disk does not match the node that opened the store.
    IdentityMismatch { expected: String, found: String },
    /// An operation requiring an open store was called before `open_store`.
    NoStore,
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Rusqlite(e) => write!(f, "sqlite error: {e}"),
            StoreError::Serde(e) => write!(f, "serialization error: {e}"),
            StoreError::IdentityMismatch { expected, found } => {
                write!(
                    f,
                    "identity mismatch: store has '{found}', node is '{expected}'"
                )
            }
            StoreError::NoStore => write!(f, "store not opened (call open_store first)"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError::Rusqlite(e)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(e: serde_json::Error) -> Self {
        StoreError::Serde(e.to_string())
    }
}

/// A local-first, file-backed store for exactly one node.
pub struct Store {
    conn: Connection,
    #[allow(dead_code)]
    path: PathBuf,
}

impl Store {
    /// Open (creating if needed) the SQLite database at `path` and claim it as this
    /// node's store. A DB is owned by exactly one node (the local-first invariant): the
    /// first opener records its identity; any later opener with a *different* identitiy is
    /// rejected immediately (`IdentityMismatch`). This prevents one node silently loading
    /// another node's custody queue.
    pub fn open(
        path: &Path,
        eid: &str,
        pk: &[u8],
        hybrid_pk: &[u8],
        sk: &[u8],
    ) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL").ok();
        conn.execute_batch(SCHEMA)?;

        let existing: rusqlite::Result<(String, Vec<u8>)> =
            conn.query_row("SELECT owner_eid, pk FROM identity LIMIT 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            });
        match existing {
            Ok((owner_eid, owner_pk)) => {
                // Owned by another identity → fault, do not clobber.
                if owner_eid != eid || owner_pk != pk {
                    return Err(StoreError::IdentityMismatch {
                        expected: eid.to_string(),
                        found: owner_eid,
                    });
                }
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                // First open: record this node as the sole owner.
                conn.execute(
                    "INSERT INTO identity (owner_eid, pk, hybrid_pk, sk, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![eid, pk, hybrid_pk, sk, now_ts()],
                )?;
            }
            Err(e) => return Err(StoreError::Rusqlite(e)),
        }

        Ok(Store {
            conn,
            path: path.to_path_buf(),
        })
    }

    /// Verify the on-disk identity matches this node before loading custody/seen.
    fn check_identity(&self, eid: &str, pk: &[u8]) -> Result<(), StoreError> {
        let row: (String, Vec<u8>) = self
            .conn
            .query_row("SELECT owner_eid, pk FROM identity LIMIT 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => StoreError::NoStore,
                e => StoreError::Rusqlite(e),
            })?;
        if row.0 != eid || row.1 != pk {
            return Err(StoreError::IdentityMismatch {
                expected: eid.to_string(),
                found: row.0,
            });
        }
        Ok(())
    }

    /// Atomically persist the custody queue (in order) and the replay-dedup set.
    pub fn save(
        &mut self,
        custody: &[Bundle],
        seen: &std::collections::HashSet<(String, u64)>,
    ) -> Result<(), StoreError> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM custody", [])?;
        tx.execute("DELETE FROM seen", [])?;
        for (idx, b) in custody.iter().enumerate() {
            tx.execute(
                "INSERT INTO custody
                   (idx, source, dest, sender_pk, sender_hybrid_pk, creation_ts, lifetime, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    idx as i64,
                    &b.source,
                    &b.dest,
                    &b.sender_pk,
                    &b.sender_hybrid_pk,
                    b.creation_ts as i64,
                    b.lifetime as i64,
                    &b.payload,
                ],
            )?;
        }
        for (source, creation_ts) in seen {
            tx.execute(
                "INSERT OR IGNORE INTO seen (source, creation_ts) VALUES (?1, ?2)",
                rusqlite::params![source, *creation_ts as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Load custody + seen from disk, validating each persisted envelope payload so a
    /// corrupted row fails loudly (returns `Err`) rather than restoring garbage custody.
    #[allow(clippy::type_complexity)]
    pub fn load(
        &self,
        eid: &str,
        pk: &[u8],
    ) -> Result<(Vec<Bundle>, std::collections::HashSet<(String, u64)>), StoreError> {
        self.check_identity(eid, pk)?;

        let mut stmt = self
            .conn
            .prepare("SELECT source, dest, sender_pk, sender_hybrid_pk, creation_ts, lifetime, payload FROM custody ORDER BY idx ASC")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Vec<u8>>(2)?,
                r.get::<_, Vec<u8>>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, Vec<u8>>(6)?,
            ))
        })?;

        let mut custody = Vec::new();
        for row in rows {
            let (source, dest, sender_pk, sender_hybrid_pk, creation_ts, lifetime, payload) = row?;
            // Integrity gate: the persisted envelope must deserialize, otherwise the
            // custody row is corrupted and load must fail (RED: corrupted DB row).
            let _env: SignedEnvelope = serde_json::from_slice(&payload)
                .map_err(|_| StoreError::Serde("corrupted custody payload".into()))?;
            custody.push(Bundle {
                source,
                dest,
                sender_pk,
                sender_hybrid_pk,
                creation_ts: creation_ts as u64,
                lifetime: lifetime as u64,
                payload,
            });
        }

        let mut seen = std::collections::HashSet::new();
        let mut seen_stmt = self.conn.prepare("SELECT source, creation_ts FROM seen")?;
        let seen_rows =
            seen_stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        for row in seen_rows {
            let (source, creation_ts) = row?;
            seen.insert((source, creation_ts as u64));
        }

        Ok((custody, seen))
    }

    /// Raw connection handle — used by tests to inject a corrupted row.
    #[cfg(test)]
    pub fn test_conn(&mut self) -> &mut Connection {
        &mut self.conn
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS identity (
    owner_eid   TEXT NOT NULL,
    pk          BLOB NOT NULL,
    hybrid_pk   BLOB NOT NULL,
    sk          BLOB NOT NULL,
    updated_at  INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS custody (
    idx             INTEGER PRIMARY KEY,
    source          TEXT NOT NULL,
    dest            TEXT NOT NULL,
    sender_pk       BLOB NOT NULL,
    sender_hybrid_pk BLOB NOT NULL,
    creation_ts     INTEGER NOT NULL,
    lifetime        INTEGER NOT NULL,
    payload         BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS seen (
    source      TEXT NOT NULL,
    creation_ts INTEGER NOT NULL,
    PRIMARY KEY (source, creation_ts)
);
";

// ── RED+GREEN durability tests ─────────────────────────────────────────────
// Every gate fails if the persistence logic breaks.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Node, SignedEnvelope};
    use dowiz_kernel::pq::envelope::open as open_envelope;

    fn temp_db(name: &str) -> PathBuf {
        // Unique per test (name) + process to survive parallel runs; cleaned up after.
        let p =
            std::env::temp_dir().join(format!("dowiz_store_{}_{}.db", std::process::id(), name));
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(format!("{}-wal", p.display()));
        let _ = std::fs::remove_file(format!("{}-shm", p.display()));
        p
    }

    const SEED_A: [u8; 32] = [1u8; 32];
    const SEED_B: [u8; 32] = [2u8; 32];

    #[test]
    fn green_open_store_accept_save_restart_load_restores_custody() {
        let path = temp_db("green_restart");
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);

        b.open_store(&path).expect("open store");
        let bundle = a.make_bundle("dtn://b", b"hello courier", 1000, 3600);
        assert!(b.accept(bundle).is_ok());
        assert_eq!(b.custody_len(), 1);
        b.save_state().expect("save state");

        // Simulate process restart: drop the node, rebuild deterministically, reopen.
        drop(b);
        let mut b2 = Node::new("dtn://b", &SEED_B, 1000);
        b2.open_store(&path).expect("reopen store");
        b2.load_state().expect("load state");
        assert_eq!(b2.custody_len(), 1, "custody must survive restart");

        // The restored bundle must still be a valid, verifiable envelope.
        let restored = b2.peek_custody(0).expect("bundle present");
        let env: SignedEnvelope =
            serde_json::from_slice(&restored.payload).expect("envelope deserializes");
        assert!(open_envelope(&env, &restored.sender_pk).is_ok());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn green_save_persists_replay_dedup_set() {
        let path = temp_db("green_dedup");
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        b.open_store(&path).expect("open store");

        let bundle = a.make_bundle("dtn://b", b"dup", 1000, 3600);
        assert!(b.accept(bundle.clone()).is_ok());
        // Replaying the same (source, creation_ts) is rejected in-memory...
        assert_eq!(b.accept(bundle), Err("replay"));
        b.save_state().expect("save state");

        // ...and after restart the dedup set is restored, so replay is still rejected.
        drop(b);
        let mut b2 = Node::new("dtn://b", &SEED_B, 1000);
        b2.open_store(&path).expect("reopen store");
        b2.load_state().expect("load state");
        let replay = a.make_bundle("dtn://b", b"dup", 1000, 3600);
        assert_eq!(
            b2.accept(replay),
            Err("replay"),
            "dedup must survive restart"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn red_corrupted_custody_row_fails_to_load() {
        let path = temp_db("red_corrupt");
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        b.open_store(&path).expect("open store");

        let bundle = a.make_bundle("dtn://b", b"legit", 1000, 3600);
        assert!(b.accept(bundle).is_ok());
        b.save_state().expect("save state");
        drop(b);

        // Reopen as B, corrupt the persisted payload blob directly through raw_store,
        // then load_state must fail closed on the corrupted row.
        let mut b2 = Node::new("dtn://b", &SEED_B, 1000);
        b2.open_store(&path).expect("reopen store");
        b2.raw_store()
            .expect("store open")
            .test_conn()
            .execute(
                "UPDATE custody SET payload = X'00deadbeef' WHERE idx = 0",
                [],
            )
            .expect("corrupt row");

        // load_state must return Err on the corrupted row instead of restoring garbage.
        let res = b2.load_state();
        assert!(res.is_err(), "corrupted DB row must fail to load");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn red_identity_mismatch_rejected_on_load() {
        let path = temp_db("red_identity");
        let a = Node::new("dtn://a", &SEED_A, 1000);
        let mut b = Node::new("dtn://b", &SEED_B, 1000);
        b.open_store(&path).expect("open store");
        let bundle = a.make_bundle("dtn://b", b"x", 1000, 3600);
        assert!(b.accept(bundle).is_ok());
        b.save_state().expect("save state");
        drop(b);

        // A different node (C) tries to open B's store — ownership is rejected at open().
        let mut c = Node::new("dtn://c", &[9u8; 32], 1000);
        let open_res = c.open_store(&path);
        assert!(
            open_res.is_err(),
            "wrong node must not claim another node's store"
        );

        let _ = std::fs::remove_file(&path);
    }
}
