//! An append-only, content-addressed event log in a bebop store.
//!
//! Shaped for dowiz's `EventStore` (`crates/dowiz-core/src/event_log.rs:191`), but this module
//! stays dependency-free and speaks raw records; the kernel shim maps `MeshEvent` onto it.
//!
//! Unlike the KV layout, this does NOT rewrite the world on every write. Each record is its own
//! object carrying an object-relative ref to its predecessor -- the same node-chain shape
//! `scrash.bp` uses -- so an append allocates ONE object, relinks the root, and commits. That
//! is O(1) per insert, which is what an event log needs and what bebop's append-only arena is
//! already built for.
//!
//! Record object EV, `15 + payload_len` cells:
//!   0        actor_seq
//!   1        payload_len
//!   2        ref to the previous EV (object-relative, 0 = genesis)
//!   3..6     content-id, 32 bytes as 4 little-endian cells
//!   7..10    prev content-id
//!   11..14   actor_pubkey
//!   15..     payload bytes, one per cell
//!
//! Root EVLOG, 7 cells: {n, ref LAST, has_tip, tip0..tip3}

use crate::{Store, StoreError};

/// `st_digest("EVLOG{i64,ref EV,i64,i64,i64,i64,i64}")` — see kv.rs on how digests are pinned.
pub const DIGEST_EVLOG_ROOT: i64 = 0x4556_4C47;
/// `st_digest("EV{...}")`.
pub const DIGEST_EV: i64 = 0x4556_5F52;

/// One event as this log stores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    pub id: [u8; 32],
    pub prev: [u8; 32],
    pub actor_pubkey: [u8; 32],
    pub actor_seq: u64,
    pub payload: Vec<u8>,
}

fn b32_to_cells(b: &[u8; 32]) -> [i64; 4] {
    let mut out = [0i64; 4];
    for i in 0..4 {
        let mut w = [0u8; 8];
        w.copy_from_slice(&b[i * 8..i * 8 + 8]);
        out[i] = i64::from_le_bytes(w);
    }
    out
}

fn cells_to_b32(c: &[i64]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..i * 8 + 8].copy_from_slice(&c[i].to_le_bytes());
    }
    out
}

/// The log's read/write handle.
pub struct EvLog;

impl EvLog {
    /// Stage the empty EVLOG schema. Shared by both commit paths so the layout
    /// has exactly one definition.
    fn stage_init(st: &mut Store) -> Result<(crate::Tx, usize), StoreError> {
        let mut tx = st.begin()?;
        let root = st.alloc(&mut tx, 7, DIGEST_EVLOG_ROOT)?;
        for i in 0..7 { st.put_cell(root, i, 0); }
        st.seal(root);
        Ok((tx, root))
    }

    /// Create the empty EVLOG schema in a fresh store.
    pub fn init(st: &mut Store, path: &str) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_init(st)?;
        st.commit(&tx, root, path)
    }

    /// `init` with no filesystem — for a store that lives in object storage.
    pub fn init_bytes(st: &mut Store) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_init(st)?;
        Ok(st.commit_bytes(&tx, root))
    }

    /// Number of records.
    pub fn len(st: &Store) -> usize {
        st.root().map(|r| st.get(r, 0) as usize).unwrap_or(0)
    }

    /// The chain tip, if one has been set.
    pub fn tip(st: &Store) -> Option<[u8; 32]> {
        let r = st.root()?;
        if st.get(r, 2) == 0 { return None; }
        let c: Vec<i64> = (3..7).map(|i| st.get(r, i)).collect();
        Some(cells_to_b32(&c))
    }

    /// Read a record object.
    fn read_at(st: &Store, obj: usize) -> Record {
        let seq = st.get(obj, 0) as u64;
        let plen = st.get(obj, 1) as usize;
        let idc: Vec<i64> = (3..7).map(|i| st.get(obj, i)).collect();
        let prevc: Vec<i64> = (7..11).map(|i| st.get(obj, i)).collect();
        let pkc: Vec<i64> = (11..15).map(|i| st.get(obj, i)).collect();
        let payload: Vec<u8> = (0..plen).map(|j| st.get(obj, 15 + j) as u8).collect();
        Record {
            id: cells_to_b32(&idc),
            prev: cells_to_b32(&prevc),
            actor_pubkey: cells_to_b32(&pkc),
            actor_seq: seq,
            payload,
        }
    }

    /// Walk the chain newest-first. Returns records in reverse insertion order.
    pub fn walk(st: &Store) -> Vec<Record> {
        let mut out = Vec::new();
        let Some(root) = st.root() else { return out };
        let mut cur = st.follow(root, 1);
        while let Some(obj) = cur {
            out.push(Self::read_at(st, obj));
            cur = st.follow(obj, 2);
        }
        out
    }

    /// Stage one appended record. The record layout lives HERE and nowhere else:
    /// two copies of it would drift, and a drifted layout reads as corruption.
    fn stage_append(st: &mut Store, rec: &Record) -> Result<(crate::Tx, usize), StoreError> {
        let old_root = st.root().ok_or(StoreError::NoSuperblock)?;
        let n = st.get(old_root, 0);
        let has_tip = st.get(old_root, 2);
        let tipc: Vec<i64> = (3..7).map(|i| st.get(old_root, i)).collect();
        let last = st.follow(old_root, 1);

        let mut tx = st.begin()?;
        let ev = st.alloc(&mut tx, 15 + rec.payload.len() as i64, DIGEST_EV)?;
        st.put_cell(ev, 0, rec.actor_seq as i64);
        st.put_cell(ev, 1, rec.payload.len() as i64);
        match last {
            Some(prev_obj) => st.link(ev, 2, prev_obj),
            None => st.put_cell(ev, 2, 0),
        }
        for (i, c) in b32_to_cells(&rec.id).iter().enumerate() { st.put_cell(ev, 3 + i, *c); }
        for (i, c) in b32_to_cells(&rec.prev).iter().enumerate() { st.put_cell(ev, 7 + i, *c); }
        for (i, c) in b32_to_cells(&rec.actor_pubkey).iter().enumerate() { st.put_cell(ev, 11 + i, *c); }
        for (j, b) in rec.payload.iter().enumerate() { st.put_cell(ev, 15 + j, *b as i64); }
        st.seal(ev);

        let root = st.alloc(&mut tx, 7, DIGEST_EVLOG_ROOT)?;
        st.put_cell(root, 0, n + 1);
        st.link(root, 1, ev);
        st.put_cell(root, 2, has_tip);
        for i in 0..4 { st.put_cell(root, 3 + i, tipc[i]); }
        st.seal(root);
        Ok((tx, root))
    }

    /// Append one record and commit. Allocates a SINGLE object and relinks the
    /// root — O(1), which is what an event log needs.
    pub fn append(st: &mut Store, path: &str, rec: &Record) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_append(st, rec)?;
        st.commit(&tx, root, path)
    }

    /// `append` with no filesystem.
    pub fn append_bytes(st: &mut Store, rec: &Record) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_append(st, rec)?;
        Ok(st.commit_bytes(&tx, root))
    }

    /// Stage a chain-tip change.
    fn stage_set_tip(st: &mut Store, id: &[u8; 32]) -> Result<(crate::Tx, usize), StoreError> {
        let old_root = st.root().ok_or(StoreError::NoSuperblock)?;
        let n = st.get(old_root, 0);
        let last = st.follow(old_root, 1);
        let mut tx = st.begin()?;
        let root = st.alloc(&mut tx, 7, DIGEST_EVLOG_ROOT)?;
        st.put_cell(root, 0, n);
        match last {
            Some(o) => st.link(root, 1, o),
            None => st.put_cell(root, 1, 0),
        }
        st.put_cell(root, 2, 1);
        for (i, c) in b32_to_cells(id).iter().enumerate() { st.put_cell(root, 3 + i, *c); }
        st.seal(root);
        Ok((tx, root))
    }

    /// Set the chain tip and commit.
    pub fn set_tip(st: &mut Store, path: &str, id: &[u8; 32]) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_set_tip(st, id)?;
        st.commit(&tx, root, path)
    }

    /// `set_tip` with no filesystem.
    pub fn set_tip_bytes(st: &mut Store, id: &[u8; 32]) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_set_tip(st, id)?;
        Ok(st.commit_bytes(&tx, root))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(n: u8, payload: &[u8]) -> Record {
        Record { id: [n; 32], prev: [n.wrapping_sub(1); 32], actor_pubkey: [0xAA; 32],
                 actor_seq: n as u64, payload: payload.to_vec() }
    }

    /// The byte path and the file path must produce the SAME log. If they ever
    /// diverge the format has two definitions, and a store written on a Worker
    /// would not open on a hub.
    #[test]
    fn byte_path_produces_the_same_log_as_the_file_path() {
        let mut st = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut st).unwrap();
        for (i, body) in [b"alpha".as_slice(), b"beta", b"gamma"].iter().enumerate() {
            EvLog::append_bytes(&mut st, &rec(i as u8 + 1, body)).unwrap();
        }
        EvLog::set_tip_bytes(&mut st, &[7u8; 32]).unwrap();

        // Hand the byte image to the ORDINARY file reader.
        let p = std::env::temp_dir().join("bebop_evlog_bytes.store");
        let p = p.to_str().unwrap();
        std::fs::write(p, st.to_bytes()).unwrap();
        let reopened = Store::open(p).unwrap();

        assert_eq!(EvLog::len(&reopened), 3);
        let got = EvLog::walk(&reopened);
        assert_eq!(got.len(), 3, "the chain must reach every record");
        assert_eq!(got[0].payload, b"gamma", "newest first");
        assert_eq!(got[2].payload, b"alpha");
        assert_eq!(got[0].id, [3u8; 32], "32-byte ids survive the byte trip");
        assert_eq!(EvLog::tip(&reopened), Some([7u8; 32]));
        let _ = std::fs::remove_file(p);
    }

    /// An append must stay O(1): one object, not a rewrite of the world. Measured
    /// as arena growth, because that is the thing that would betray a rewrite.
    #[test]
    fn byte_append_is_constant_cost() {
        let mut st = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut st).unwrap();
        let mut prev_used = 0i64;
        let mut deltas = Vec::new();
        for i in 0..8u8 {
            EvLog::append_bytes(&mut st, &rec(i + 1, b"x")).unwrap();
            let used = st.pick().unwrap().arena_used;
            if i > 0 { deltas.push(used - prev_used); }
            prev_used = used;
        }
        assert!(deltas.windows(2).all(|w| w[0] == w[1]),
                "append cost must not grow with the log: {deltas:?}");
    }

    #[test]
    fn append_walk_roundtrip() {
        let p = std::env::temp_dir().join("bebop_evlog_roundtrip.store");
        let p = p.to_str().unwrap();
        let mut st = Store::create(p, 1 << 20).unwrap();
        EvLog::init(&mut st, p).unwrap();

        for (i, body) in [b"alpha".as_slice(), b"beta", b"gamma"].iter().enumerate() {
            let mut st = Store::open(p).unwrap();
            EvLog::append(&mut st, p, &rec(i as u8 + 1, body)).unwrap();
        }
        let st = Store::open(p).unwrap();
        assert_eq!(EvLog::len(&st), 3);
        let got = EvLog::walk(&st);
        assert_eq!(got.len(), 3, "the chain must reach every record");
        // newest first
        assert_eq!(got[0].payload, b"gamma");
        assert_eq!(got[1].payload, b"beta");
        assert_eq!(got[2].payload, b"alpha");
        assert_eq!(got[0].actor_seq, 3);
        assert_eq!(got[0].id, [3u8; 32], "32-byte ids survive the cell round-trip");
        assert_eq!(got[2].actor_pubkey, [0xAAu8; 32]);
        assert_eq!(EvLog::tip(&st), None, "no tip until one is set");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn tip_persists() {
        let p = std::env::temp_dir().join("bebop_evlog_tip.store");
        let p = p.to_str().unwrap();
        let mut st = Store::create(p, 1 << 20).unwrap();
        EvLog::init(&mut st, p).unwrap();
        let mut st = Store::open(p).unwrap();
        EvLog::append(&mut st, p, &rec(1, b"one")).unwrap();
        let mut st = Store::open(p).unwrap();
        EvLog::set_tip(&mut st, p, &[7u8; 32]).unwrap();
        let st = Store::open(p).unwrap();
        assert_eq!(EvLog::tip(&st), Some([7u8; 32]));
        assert_eq!(EvLog::len(&st), 1, "setting the tip must not disturb the record count");
        assert_eq!(EvLog::walk(&st).len(), 1, "nor the chain");
        let _ = std::fs::remove_file(p);
    }

    /// The property that separates this from the KV layout: appending record k+1 must cost the
    /// same arena as appending record k. An eager rewrite would grow quadratically.
    #[test]
    fn append_is_constant_cost() {
        let p = std::env::temp_dir().join("bebop_evlog_growth.store");
        let p = p.to_str().unwrap();
        let mut st = Store::create(p, 4 << 20).unwrap();
        EvLog::init(&mut st, p).unwrap();
        let mut deltas = Vec::new();
        let mut prev_used = Store::open(p).unwrap().pick().unwrap().arena_used;
        for i in 0..40u8 {
            let mut st = Store::open(p).unwrap();
            EvLog::append(&mut st, p, &rec(i, b"payload-of-fixed-size")).unwrap();
            let used = Store::open(p).unwrap().pick().unwrap().arena_used;
            deltas.push(used - prev_used);
            prev_used = used;
        }
        let first = deltas[0];
        assert!(deltas.iter().all(|d| *d == first),
                "arena growth per append must be constant, got {deltas:?}");
        // 15 header cells + 21 payload bytes + 2 object header cells, plus the 7+2 root
        assert_eq!(first, (15 + 21 + 2) + (7 + 2), "unexpected per-append cost");
        let st = Store::open(p).unwrap();
        assert_eq!(EvLog::len(&st), 40);
        assert_eq!(EvLog::walk(&st).len(), 40);
        let _ = std::fs::remove_file(p);
    }
}
