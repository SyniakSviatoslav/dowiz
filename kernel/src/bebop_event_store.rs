//! `bebop_event_store` — dowiz's [`EventStore`] backed by bebop's object store.
//!
//! This is the seam `ADR-0008` ("per-node local SQLite at rest", `Status: PROPOSED`) and
//! `DECISIONS.md` D1 ("each node running the Rust/WASM kernel + a local SQLite DB") describe.
//! No SQLite dependency was ever added to back it, and the only durable `EventStore` in the
//! tree — the JSONL [`FileEventStore`](crate::brain::hydra::FileEventStore) — is reachable
//! only from tests. This fills that slot with bebop instead: one file, no SQL, no server.
//!
//! Compiled ONLY under the `bebopdb` feature, so the default kernel build and the wasm chain
//! are untouched.
//!
//! Durability comes from bebop's commit protocol rather than from an fsync of an appended
//! line: each event is ONE new object plus a relinked root, committed by a superblock toggle,
//! with the writes ordered objects → PartTab → superblock. A crash mid-append leaves the
//! store readable at the previous generation, so a torn write cannot produce a half-event.
//!
//! Append cost is CONSTANT in the log length — each record is its own object carrying an
//! object-relative ref to its predecessor, so the log is a chain rather than a rewritten
//! array. That is the difference between this and the KV adapter in `retrieval::memory_store`.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bebop_store::evlog::{EvLog, Record};
use bebop_store::Store;
use dowiz_core::event_log::{EventStore, MeshEvent, StoreError};

/// Bebop-backed durable event store.
pub struct BebopEventStore {
    path: String,
    /// Read index rebuilt from the chain at open; keeps `contains`/`get` off the disk walk.
    index: BTreeMap<[u8; 32], MeshEvent>,
    /// Insertion order, oldest first.
    order: Vec<[u8; 32]>,
    tip: Option<[u8; 32]>,
}

fn to_mesh(r: &Record) -> MeshEvent {
    MeshEvent {
        prev: r.prev,
        actor_pubkey: r.actor_pubkey,
        actor_seq: r.actor_seq,
        payload: r.payload.clone(),
    }
}

impl BebopEventStore {
    /// Create a fresh store file with an empty log.
    pub fn create(path: &str, size_bytes: usize) -> Result<Self, StoreError> {
        let mut st = Store::create(path, size_bytes).map_err(|e| StoreError::Open(e.to_string()))?;
        EvLog::init(&mut st, path).map_err(|e| StoreError::Write(alloc::format!("{e:?}")))?;
        Self::open(path)
    }

    /// Open an existing store, or create one if the file is absent or has no valid
    /// superblock. This is what a composition root wants: a node that boots for the first
    /// time must not have to be initialised by hand.
    pub fn open_or_create(path: &str, size_bytes: usize) -> Result<Self, StoreError> {
        let fresh = match Store::open(path) {
            Err(_) => true,
            Ok(st) => st.root().is_none(),
        };
        if fresh { Self::create(path, size_bytes) } else { Self::open(path) }
    }

    /// Open an existing store and rebuild the read index by walking the chain.
    pub fn open(path: &str) -> Result<Self, StoreError> {
        let st = Store::open(path).map_err(|e| StoreError::Open(e.to_string()))?;
        let mut recs = EvLog::walk(&st);
        recs.reverse(); // walk yields newest-first; keep insertion order
        let mut index = BTreeMap::new();
        let mut order = Vec::with_capacity(recs.len());
        for r in &recs {
            index.insert(r.id, to_mesh(r));
            order.push(r.id);
        }
        Ok(BebopEventStore { path: path.to_string(), index, order, tip: EvLog::tip(&st) })
    }

    /// The generation the underlying store is at — one per committed append.
    pub fn generation(&self) -> Result<i64, StoreError> {
        let st = Store::open(&self.path).map_err(|e| StoreError::Open(e.to_string()))?;
        st.pick().map(|sb| sb.generation).ok_or_else(|| StoreError::Open("no valid superblock".to_string()))
    }
}

impl EventStore for BebopEventStore {
    fn contains(&self, id: &[u8; 32]) -> bool {
        self.index.contains_key(id)
    }

    fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError> {
        // Content-addressed and therefore idempotent: re-inserting the same id is a no-op,
        // never a second copy on disk.
        if self.index.contains_key(&id) {
            return Ok(());
        }
        let rec = Record {
            id,
            prev: ev.prev,
            actor_pubkey: ev.actor_pubkey,
            actor_seq: ev.actor_seq,
            payload: ev.payload.clone(),
        };
        let mut st = Store::open(&self.path).map_err(|e| StoreError::Open(e.to_string()))?;
        EvLog::append(&mut st, &self.path, &rec)
            .map_err(|e| StoreError::Sync(alloc::format!("{e:?}")))?;
        self.index.insert(id, ev);
        self.order.push(id);
        Ok(())
    }

    fn get(&self, id: &[u8; 32]) -> Option<MeshEvent> {
        self.index.get(id).cloned()
    }

    fn len(&self) -> usize {
        self.order.len()
    }

    fn tip(&self) -> Option<[u8; 32]> {
        self.tip
    }

    fn set_tip(&mut self, id: [u8; 32]) {
        if let Ok(mut st) = Store::open(&self.path) {
            let _ = EvLog::set_tip(&mut st, &self.path, &id);
        }
        self.tip = Some(id);
    }

    fn ids(&self) -> Vec<[u8; 32]> {
        self.order.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(seq: u64, body: &[u8]) -> MeshEvent {
        MeshEvent { prev: [0u8; 32], actor_pubkey: [9u8; 32], actor_seq: seq, payload: body.to_vec() }
    }

    fn tmp(name: &str) -> String {
        std::env::temp_dir().join(name).to_str().unwrap().to_string()
    }

    /// Events survive a reopen -- which is the whole point, since every order-carrying
    /// surface in this repo today holds state in a HashMap that dies with the process.
    #[test]
    fn events_survive_reopen() {
        let p = tmp("bebop_evstore_reopen.store");
        let _ = std::fs::remove_file(&p);
        let mut s = BebopEventStore::create(&p, 1 << 20).expect("create");
        let a = ev(1, b"order/0001 -> Confirmed");
        let b = ev(2, b"order/0001 -> Preparing");
        let (ia, ib) = (a.event_id(), b.event_id());
        s.insert(ia, a.clone()).expect("insert a");
        s.insert(ib, b.clone()).expect("insert b");
        assert_eq!(s.len(), 2);

        // a fresh handle, as a restarted process would take
        let s2 = BebopEventStore::open(&p).expect("reopen");
        assert_eq!(s2.len(), 2, "both events must still be there after reopen");
        assert!(s2.contains(&ia) && s2.contains(&ib));
        assert_eq!(s2.get(&ia).unwrap(), a, "the event body round-trips byte-exactly");
        assert_eq!(s2.get(&ib).unwrap(), b);
        assert_eq!(s2.ids(), vec![ia, ib], "insertion order is preserved");
        let _ = std::fs::remove_file(&p);
    }

    /// Content-addressing means a re-insert is a no-op, not a second copy on disk.
    #[test]
    fn insert_is_idempotent() {
        let p = tmp("bebop_evstore_idem.store");
        let _ = std::fs::remove_file(&p);
        let mut s = BebopEventStore::create(&p, 1 << 20).expect("create");
        let e = ev(7, b"same");
        let id = e.event_id();
        s.insert(id, e.clone()).unwrap();
        let gen_after_first = s.generation().unwrap();
        s.insert(id, e.clone()).unwrap();
        s.insert(id, e).unwrap();
        assert_eq!(s.len(), 1, "three inserts of one content-id are one event");
        assert_eq!(s.generation().unwrap(), gen_after_first,
                   "a duplicate insert must not commit a new generation");
        let s2 = BebopEventStore::open(&p).unwrap();
        assert_eq!(s2.len(), 1);
        let _ = std::fs::remove_file(&p);
    }

    /// A different payload is a different content-id, so it IS stored.
    #[test]
    fn distinct_payloads_are_distinct_events() {
        let p = tmp("bebop_evstore_distinct.store");
        let _ = std::fs::remove_file(&p);
        let mut s = BebopEventStore::create(&p, 1 << 20).expect("create");
        let a = ev(1, b"Confirmed");
        let b = ev(1, b"Cancelled");
        assert_ne!(a.event_id(), b.event_id());
        s.insert(a.event_id(), a).unwrap();
        s.insert(b.event_id(), b).unwrap();
        assert_eq!(s.len(), 2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn tip_survives_reopen() {
        let p = tmp("bebop_evstore_tip.store");
        let _ = std::fs::remove_file(&p);
        let mut s = BebopEventStore::create(&p, 1 << 20).expect("create");
        let e = ev(1, b"x");
        let id = e.event_id();
        s.insert(id, e).unwrap();
        assert_eq!(s.tip(), None);
        s.set_tip(id);
        let s2 = BebopEventStore::open(&p).unwrap();
        assert_eq!(s2.tip(), Some(id), "the tip must be durable, not just in RAM");
        assert_eq!(s2.len(), 1, "setting the tip must not disturb the log");
        let _ = std::fs::remove_file(&p);
    }
}
