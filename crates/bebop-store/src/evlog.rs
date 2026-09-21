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
//! ── TWO VERSIONS, AND THE ROOT SAYS WHICH ──
//!
//! v1 root, 7 cells: `{n, ref LAST, has_tip, tip0..tip3}`
//! v1 record EV, `15 + payload_len` cells:
//!   0        actor_seq
//!   1        payload_len
//!   2        ref to the previous EV (object-relative, 0 = genesis)
//!   3..6     content-id, 32 bytes as 4 little-endian cells
//!   7..10    prev content-id
//!   11..14   actor_pubkey
//!   15..     payload bytes, ONE PER CELL -- eight bytes of arena for one byte of event
//!
//! v2 root, 8 cells: `{n, ref LAST, has_tip, tip0..tip3, VERSION}`
//! v2 record EV, `12 + (4 if the actor is named) + ceil(payload_len / 8)` cells:
//!   0        actor_seq
//!   1        payload_len IN BYTES
//!   2        ref to the previous EV
//!   3..6     content-id
//!   7..10    prev content-id
//!   11       flags: bit 0 = an actor_pubkey follows
//!   12..     the actor_pubkey (4 cells) when that bit is set, then the payload
//!            PACKED EIGHT BYTES TO A CELL, little-endian, last cell zero-filled
//!
//! WHY v2 EXISTS. Measured on the live venue: 583 cells -- 4.66 KB -- per event, for a
//! payload of a few hundred bytes. One byte per eight-byte cell is an 8x amplification, and
//! the 32-byte actor key every dowiz caller leaves zero cost four cells of nothing each.
//! A 330-byte event is 345 cells in v1 and 54 in v2.
//!
//! HOW THE TWO LIVE TOGETHER. The version is read off the ROOT, so a store answers for itself:
//! a v1 image keeps being appended to in v1 -- its records are v1 and a mixed chain would be
//! unreadable -- and every image created from now on is v2. An old image becomes v2 when its
//! owner rebuilds it, which `Hub::grow` and `StockLog::grow` do on the next doubling: they
//! init a FRESH store and replay the chain into it. No migration runs, nothing is rewritten
//! in place, and there is no moment where a reader has to guess.

use crate::{Store, StoreError};

/// `st_digest("EVLOG{i64,ref EV,i64,i64,i64,i64,i64}")` — see kv.rs on how digests are pinned.
pub const DIGEST_EVLOG_ROOT: i64 = 0x4556_4C47;
/// `st_digest("EV{...}")`.
pub const DIGEST_EV: i64 = 0x4556_5F52;

/// Pack payload bytes into cells, 8 bytes per cell, little-endian.
/// The last cell is zero-padded if the payload is not a multiple of 8.
/// This is the packing format a v2 record will use; nothing writes it yet.
pub fn pack_payload(bytes: &[u8]) -> Vec<i64> {
    let n_cells = (bytes.len() + 7) / 8;
    let mut cells = vec![0i64; n_cells];
    for i in 0..bytes.len() {
        let cell_idx = i / 8;
        let byte_idx = i % 8;
        cells[cell_idx] |= (bytes[i] as i64) << (byte_idx * 8);
    }
    cells
}

/// Unpack cells back to payload bytes. `len` is the payload's true length,
/// which the header carries; cells beyond len are discarded.
pub fn unpack_payload(cells: &[i64], len: usize) -> Vec<u8> {
    // A `len` longer than the cells hold is a corrupt header, not a panic: the
    // caller gets the bytes that are really there and can refuse them itself.
    let len = len.min(cells.len() * 8);
    let mut bytes = Vec::with_capacity(len);
    for i in 0..len {
        let cell_idx = i / 8;
        let byte_idx = i % 8;
        let b = ((cells[cell_idx] >> (byte_idx * 8)) & 0xFF) as u8;
        bytes.push(b);
    }
    bytes
}

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

/// The version this module writes. A store says its own version in its root;
/// this is only what a FRESH log is created as.
pub const VERSION: i64 = 2;

/// Cells in a v1 root, and in a v2 one. The extra cell is the version itself.
const ROOT_V1: i64 = 7;
const ROOT_V2: i64 = 8;

/// v1 record header, before the payload's one-byte-per-cell tail.
const HEAD_V1: i64 = 15;
/// v2 record header, before the optional actor key and the packed payload.
const HEAD_V2: i64 = 12;
/// Bit 0 of cell 11: an `actor_pubkey` follows the header.
const FLAG_ACTOR: i64 = 1;

/// The log's read/write handle.
pub struct EvLog;

impl EvLog {
    /// Which version this store's log is written in.
    ///
    /// READ OFF THE ROOT, never guessed from a record: a record carries no
    /// version of its own, and it does not need one -- a log is entirely one
    /// version or entirely the other, because an append writes what the root
    /// says. A root with no version cell is v1, which is exactly what every
    /// image written before this change looks like.
    pub fn version(st: &Store) -> i64 {
        let Some(root) = st.root() else { return VERSION };
        if st.obj_len(root) >= ROOT_V2 {
            let v = st.get(root, 7);
            if v > 0 {
                return v;
            }
        }
        1
    }

    /// Stage the empty EVLOG schema. Shared by both commit paths so the layout
    /// has exactly one definition.
    fn stage_init(st: &mut Store) -> Result<(crate::Tx, usize), StoreError> {
        let mut tx = st.begin()?;
        let root = st.alloc(&mut tx, ROOT_V2, DIGEST_EVLOG_ROOT)?;
        for i in 0..ROOT_V2 as usize {
            st.put_cell(root, i, 0);
        }
        st.put_cell(root, 7, VERSION);
        st.seal(root);
        Ok((tx, root))
    }

    /// Stage an empty V1 schema — for the tests that have to prove an old image
    /// still reads, and for nothing else.
    #[cfg(test)]
    fn stage_init_v1(st: &mut Store) -> Result<(crate::Tx, usize), StoreError> {
        let mut tx = st.begin()?;
        let root = st.alloc(&mut tx, ROOT_V1, DIGEST_EVLOG_ROOT)?;
        for i in 0..ROOT_V1 as usize {
            st.put_cell(root, i, 0);
        }
        st.seal(root);
        Ok((tx, root))
    }

    #[cfg(test)]
    fn init_v1_bytes(st: &mut Store) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_init_v1(st)?;
        Ok(st.commit_bytes(&tx, root))
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
        if st.get(r, 2) == 0 {
            return None;
        }
        let c: Vec<i64> = (3..7).map(|i| st.get(r, i)).collect();
        Some(cells_to_b32(&c))
    }

    /// How many cells a record of this payload takes, in the version this
    /// store is written in. The one place the size is computed, so `alloc` and
    /// any measurement of it cannot disagree.
    fn record_cells(version: i64, rec: &Record) -> i64 {
        if version >= 2 {
            let named = rec.actor_pubkey != [0u8; 32];
            HEAD_V2 + if named { 4 } else { 0 } + rec.payload.len().div_ceil(8) as i64
        } else {
            HEAD_V1 + rec.payload.len() as i64
        }
    }

    /// Read a record object, in the version the log is written in.
    fn read_at(st: &Store, version: i64, obj: usize) -> Record {
        let seq = st.get(obj, 0) as u64;
        let plen = st.get(obj, 1) as usize;
        let idc: Vec<i64> = (3..7).map(|i| st.get(obj, i)).collect();
        let prevc: Vec<i64> = (7..11).map(|i| st.get(obj, i)).collect();
        if version >= 2 {
            let flags = st.get(obj, 11);
            // THE FLAG IS A CLAIM AND THE OBJECT'S LENGTH IS THE FACT. A
            // damaged record with the bit set and nothing behind it would have
            // read four cells past its own end -- and `Store::get` is an
            // unchecked index, so at the end of an arena that is a panic
            // rather than a report. The payload below has been bounded since
            // it was written; the key was not.
            let named = flags & FLAG_ACTOR != 0
                && st.obj_cells(obj) >= (HEAD_V2 + 4) as usize;
            let at = HEAD_V2 as usize;
            let (actor, payload_at) = if named {
                let pkc: Vec<i64> = (at..at + 4).map(|i| st.get(obj, i)).collect();
                (cells_to_b32(&pkc), at + 4)
            } else {
                ([0u8; 32], at)
            };
            // What the object actually holds, not what the header claims: a
            // truncated record must hand back the bytes that are there rather
            // than read past its own end.
            // `obj_cells`, NOT `obj_len`: the length is a 32-bit field, so a
            // damaged one claims up to four billion cells and this line
            // allocated every one of them. Bounded by the image, a corrupt
            // record is a SHORT record -- which the layer above refuses.
            let have = st.obj_cells(obj).saturating_sub(payload_at);
            let cells: Vec<i64> =
                (0..have).map(|j| st.get(obj, payload_at + j)).collect();
            Record {
                id: cells_to_b32(&idc),
                prev: cells_to_b32(&prevc),
                actor_pubkey: actor,
                actor_seq: seq,
                payload: unpack_payload(&cells, plen),
            }
        } else {
            let pkc: Vec<i64> = (11..15).map(|i| st.get(obj, i)).collect();
            // THE HEADER'S LENGTH IS A CLAIM; THE OBJECT'S LENGTH IS THE FACT.
            // v1 stores one payload byte per cell from HEAD_V1, so a record
            // holds no more bytes than it has cells behind that head. Trusting
            // the claim was not a robustness nicety: a single flipped bit in
            // cell 1 turns a 32-byte payload into a `plen` of 0x8000_0020 and
            // this line into a 2 GiB allocation. On this box that is the whole
            // app killed by the platform; on a Worker it is the isolate. The
            // v2 branch above is bounded the same way -- this is one law, and
            // `obj_cells` is where it is stated.
            //
            // A record only reachable through v1 is already corrupt (a fresh
            // log is v2 and the reader falls back to v1 only when the root's
            // version cell is damaged), so the bytes that ARE there are handed
            // back exactly like a truncated record's, for the caller to refuse.
            let have = st.obj_cells(obj).saturating_sub(HEAD_V1 as usize);
            let plen = plen.min(have);
            let payload: Vec<u8> =
                (0..plen).map(|j| st.get(obj, HEAD_V1 as usize + j) as u8).collect();
            Record {
                id: cells_to_b32(&idc),
                prev: cells_to_b32(&prevc),
                actor_pubkey: cells_to_b32(&pkc),
                actor_seq: seq,
                payload,
            }
        }
    }

    /// The most records an image of this size could possibly hold.
    ///
    /// THE CHAIN IS DATA, SO IT CAN LIE, and the shape of the lie that matters
    /// is a CYCLE: one flipped bit in a `next` ref can point a record back at
    /// one already visited, and `while let Some(obj) = cur` then walks for
    /// ever, pushing a record on every turn until the process is killed for
    /// the memory it asked for. That is not a hypothetical -- it is the same
    /// class of bug as the 8 GiB allocation, reached by a different route.
    ///
    /// A record occupies its two object-header cells plus at least a v2
    /// header, so an image of `n` cells cannot hold more than `n / 14` of
    /// them. Past that the chain is not long, it is looping.
    fn step_cap(st: &Store) -> usize {
        st.cells.len() / (2 + HEAD_V2 as usize) + 1
    }

    /// Walk the chain newest-first. Returns records in reverse insertion order.
    ///
    /// A chain that does not end stops at `step_cap`; `chain_len` is how a
    /// caller asks whether that happened, and `LogImage::load` refuses the
    /// image when it did.
    pub fn walk(st: &Store) -> Vec<Record> {
        let mut out = Vec::new();
        let Some(root) = st.root() else { return out };
        let version = Self::version(st);
        let cap = Self::step_cap(st);
        let mut cur = st.follow(root, 1);
        while let Some(obj) = cur {
            if out.len() >= cap {
                break;
            }
            out.push(Self::read_at(st, version, obj));
            cur = st.follow(obj, 2);
        }
        out
    }

    /// How many records the chain actually holds, reading none of them.
    ///
    /// `len` is what the root CLAIMS; this is what the chain DELIVERS, and the
    /// two disagreeing is the failure that has to be loud. A log that says it
    /// holds forty records and hands back two has lost thirty-eight orders,
    /// and nothing in a response built from it would say so.
    ///
    /// `None` means the chain never ended -- a cycle. It is not "very long":
    /// no honest image can chain more records than it has room for.
    pub fn chain_len(st: &Store) -> Option<usize> {
        let Some(root) = st.root() else { return Some(0) };
        let cap = Self::step_cap(st);
        let mut n = 0usize;
        let mut cur = st.follow(root, 1);
        while let Some(obj) = cur {
            n += 1;
            if n > cap {
                return None;
            }
            cur = st.follow(obj, 2);
        }
        Some(n)
    }

    /// Stage one appended record, optionally moving the chain tip in the SAME
    /// commit.
    ///
    /// THE TIP TRAVELS WITH THE RECORD when the caller asks for it. Appending
    /// and then setting the tip was two commits, and a commit is not free: a
    /// fresh root plus a 21-cell PartTab page each time, so the bookkeeping for
    /// one event cost more arena than a short event does. It is also two
    /// moments at which the log can be interrupted, and the second leaves a tip
    /// that does not name the newest record.
    ///
    /// The record layout lives HERE and nowhere else: two copies of it would
    /// drift, and a drifted layout reads as corruption.
    fn stage_append(
        st: &mut Store,
        rec: &Record,
        tip: Option<&[u8; 32]>,
    ) -> Result<(crate::Tx, usize), StoreError> {
        let old_root = st.root().ok_or(StoreError::NoSuperblock)?;
        let version = Self::version(st);
        let n = st.get(old_root, 0);
        let has_tip = st.get(old_root, 2);
        let tipc: Vec<i64> = (3..7).map(|i| st.get(old_root, i)).collect();
        let last = st.follow(old_root, 1);

        let mut tx = st.begin()?;
        let ev = st.alloc(&mut tx, Self::record_cells(version, rec), DIGEST_EV)?;
        st.put_cell(ev, 0, rec.actor_seq as i64);
        st.put_cell(ev, 1, rec.payload.len() as i64);
        match last {
            Some(prev_obj) => st.link(ev, 2, prev_obj),
            None => st.put_cell(ev, 2, 0),
        }
        for (i, c) in b32_to_cells(&rec.id).iter().enumerate() {
            st.put_cell(ev, 3 + i, *c);
        }
        for (i, c) in b32_to_cells(&rec.prev).iter().enumerate() {
            st.put_cell(ev, 7 + i, *c);
        }
        if version >= 2 {
            let named = rec.actor_pubkey != [0u8; 32];
            st.put_cell(ev, 11, if named { FLAG_ACTOR } else { 0 });
            let mut at = HEAD_V2 as usize;
            if named {
                for (i, c) in b32_to_cells(&rec.actor_pubkey).iter().enumerate() {
                    st.put_cell(ev, at + i, *c);
                }
                at += 4;
            }
            for (j, c) in pack_payload(&rec.payload).iter().enumerate() {
                st.put_cell(ev, at + j, *c);
            }
        } else {
            for (i, c) in b32_to_cells(&rec.actor_pubkey).iter().enumerate() {
                st.put_cell(ev, 11 + i, *c);
            }
            for (j, b) in rec.payload.iter().enumerate() {
                st.put_cell(ev, 15 + j, *b as i64);
            }
        }
        st.seal(ev);

        let root_cells = if version >= 2 { ROOT_V2 } else { ROOT_V1 };
        let root = st.alloc(&mut tx, root_cells, DIGEST_EVLOG_ROOT)?;
        st.put_cell(root, 0, n + 1);
        st.link(root, 1, ev);
        match tip {
            Some(id) => {
                st.put_cell(root, 2, 1);
                for (i, c) in b32_to_cells(id).iter().enumerate() {
                    st.put_cell(root, 3 + i, *c);
                }
            }
            None => {
                st.put_cell(root, 2, has_tip);
                for i in 0..4 {
                    st.put_cell(root, 3 + i, tipc[i]);
                }
            }
        }
        if version >= 2 {
            st.put_cell(root, 7, version);
        }
        st.seal(root);
        Ok((tx, root))
    }

    /// Append one record and commit. Allocates a SINGLE object and relinks the
    /// root — O(1), which is what an event log needs.
    pub fn append(st: &mut Store, path: &str, rec: &Record) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_append(st, rec, None)?;
        st.commit(&tx, root, path)
    }

    /// `append` with no filesystem.
    pub fn append_bytes(st: &mut Store, rec: &Record) -> Result<i64, StoreError> {
        let (tx, root) = Self::stage_append(st, rec, None)?;
        Ok(st.commit_bytes(&tx, root))
    }

    /// Append a record AND name it as the chain tip, in one commit.
    ///
    /// What every dowiz caller actually means by "append": the record is the
    /// newest thing in the log, so the tip is it. Two commits wrote the same
    /// answer twice and paid two PartTab pages for it.
    pub fn append_tip_bytes(st: &mut Store, rec: &Record) -> Result<i64, StoreError> {
        let id = rec.id;
        let (tx, root) = Self::stage_append(st, rec, Some(&id))?;
        Ok(st.commit_bytes(&tx, root))
    }

    /// Stage a chain-tip change.
    fn stage_set_tip(st: &mut Store, id: &[u8; 32]) -> Result<(crate::Tx, usize), StoreError> {
        let old_root = st.root().ok_or(StoreError::NoSuperblock)?;
        let version = Self::version(st);
        let n = st.get(old_root, 0);
        let last = st.follow(old_root, 1);
        let mut tx = st.begin()?;
        let root_cells = if version >= 2 { ROOT_V2 } else { ROOT_V1 };
        let root = st.alloc(&mut tx, root_cells, DIGEST_EVLOG_ROOT)?;
        st.put_cell(root, 0, n);
        match last {
            Some(o) => st.link(root, 1, o),
            None => st.put_cell(root, 1, 0),
        }
        st.put_cell(root, 2, 1);
        for (i, c) in b32_to_cells(id).iter().enumerate() {
            st.put_cell(root, 3 + i, *c);
        }
        if version >= 2 {
            st.put_cell(root, 7, version);
        }
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

    /// A CHAIN THAT POINTS BACKWARDS MUST NOT BE WALKED FOR EVER.
    ///
    /// `next` is one cell of the image like any other, so a flipped bit can
    /// aim a record at one already visited. The walk was `while let Some(obj)`
    /// with no bound: it pushed a record per turn until the process was killed
    /// for the memory it asked for. The same failure as an over-large
    /// allocation, reached by a different route -- and the only honest answer
    /// is that a chain which does not end is not a length.
    #[test]
    fn a_chain_that_loops_is_reported_rather_than_walked_for_ever() {
        let mut st = Store::create_bytes(64 * 1024);
        EvLog::init_bytes(&mut st).expect("init");
        for i in 0..5u8 {
            EvLog::append_bytes(&mut st, &rec(i, b"payload")).expect("append");
        }
        assert_eq!(EvLog::chain_len(&st), Some(5), "an honest chain reports its length");

        let root = st.root().expect("root");
        let newest = st.follow(root, 1).expect("newest");
        let older = st.follow(newest, 2).expect("second newest");
        // Aim the second record's `next` back at the first: a two-record loop.
        st.cells[older + 2 + 2] = newest as i64 - older as i64;

        assert_eq!(EvLog::chain_len(&st), None, "a chain that never ends has no length");
        assert!(
            EvLog::walk(&st).len() <= EvLog::step_cap(&st),
            "the walk must stop at the cap rather than run out of memory"
        );
    }

    /// A V1 IMAGE STILL READS, and that is the whole risk of this change.
    /// Every hub in production is v1; if a v2 reader could not read one, a
    /// deploy would blank every venue's history at once.
    #[test]
    fn a_v1_log_is_read_and_appended_to_as_v1() {
        let mut st = Store::create_bytes(1 << 20);
        EvLog::init_v1_bytes(&mut st).unwrap();
        assert_eq!(EvLog::version(&st), 1, "a root with no version cell is v1");
        for (i, body) in [b"alpha".as_slice(), b"beta", b"gamma"].iter().enumerate() {
            EvLog::append_bytes(&mut st, &rec(i as u8 + 1, body)).unwrap();
        }
        assert_eq!(EvLog::version(&st), 1, "appending to a v1 log must not change its version");
        let got = EvLog::walk(&st);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].payload, b"gamma", "newest first");
        assert_eq!(got[2].payload, b"alpha");
        assert_eq!(got[0].actor_pubkey, [0xAA; 32], "v1 always stores the actor key");
        assert_eq!(got[0].id, [3u8; 32]);

        // And a v1 record still costs what v1 records cost: 15 + payload + 2,
        // plus a 7-cell root and its header.
        let mut st2 = Store::create_bytes(1 << 20);
        EvLog::init_v1_bytes(&mut st2).unwrap();
        let before = st2.pick().unwrap().arena_used;
        EvLog::append_bytes(&mut st2, &rec(1, b"payload-of-fixed-size")).unwrap();
        let after = st2.pick().unwrap().arena_used;
        assert_eq!(after - before, (15 + 21 + 2) + (7 + 2), "v1 cost must not move");
    }

    /// A REPLAY INTO A FRESH STORE IS THE MIGRATION -- what `Hub::grow` does on
    /// the next doubling. The records come out identical and the new image is
    /// v2, which is the only way an old log ever becomes a new one.
    #[test]
    fn replaying_a_v1_log_into_a_fresh_store_makes_it_v2() {
        let mut old = Store::create_bytes(1 << 20);
        EvLog::init_v1_bytes(&mut old).unwrap();
        for i in 0..5u8 {
            EvLog::append_bytes(&mut old, &rec(i + 1, b"a realistic little payload")).unwrap();
        }
        EvLog::set_tip_bytes(&mut old, &[5u8; 32]).unwrap();

        let mut fresh = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut fresh).unwrap();
        let mut records = EvLog::walk(&old);
        records.reverse();
        for r in &records {
            EvLog::append_bytes(&mut fresh, r).unwrap();
        }
        EvLog::set_tip_bytes(&mut fresh, &[5u8; 32]).unwrap();

        assert_eq!(EvLog::version(&fresh), 2);
        assert_eq!(EvLog::walk(&fresh), EvLog::walk(&old), "every record survives the move");
        assert_eq!(EvLog::tip(&fresh), EvLog::tip(&old));
        // MEASURED AS ARENA GROWTH, not as `arena_used`: that is an absolute
        // cell index starting at the 1024-cell superblock region, so comparing
        // the raw numbers would quietly dilute the difference by a thousand
        // cells of header that neither version pays for.
        let grown = |st: &Store| st.pick().unwrap().arena_used - crate::ARENA as i64;
        let (v1, v2) = (grown(&old), grown(&fresh));
        println!("five 26-byte events: v1 {v1} cells, v2 {v2} ({}%)", v2 * 100 / v1);
        // 26 bytes is a SHORT event and the header dominates it: v1 pays
        // 15 + 26 + 2 + 9 per append, v2 pays 12 + 4 + 4 + 2 + 10. The saving
        // is a third here and four fifths at the 330-byte events a real order
        // writes -- see `cells_per_event_for_a_realistic_payload`.
        assert!(v2 * 4 < v1 * 3, "v2 {v2} cells against v1 {v1}");
    }

    /// The payload comes back byte for byte at every awkward length: empty, one
    /// short of a cell, exactly a cell, one past it.
    #[test]
    fn a_v2_payload_survives_at_every_length() {
        let mut st = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut st).unwrap();
        let sizes = [0usize, 1, 7, 8, 9, 63, 64, 65, 330];
        for (i, n) in sizes.iter().enumerate() {
            let payload: Vec<u8> = (0..*n).map(|j| (j % 251) as u8).collect();
            EvLog::append_bytes(&mut st, &rec(i as u8 + 1, &payload)).unwrap();
        }
        let got = EvLog::walk(&st);
        assert_eq!(got.len(), sizes.len());
        for (k, n) in sizes.iter().rev().enumerate() {
            let want: Vec<u8> = (0..*n).map(|j| (j % 251) as u8).collect();
            assert_eq!(got[k].payload, want, "payload of {n} bytes came back wrong");
        }
    }

    /// AN ACTOR NOBODY NAMED COSTS NOTHING. Every dowiz caller passes a zero
    /// key, and v1 wrote 32 bytes of zeros for each of them.
    #[test]
    fn an_unnamed_actor_takes_no_cells() {
        let mut st = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut st).unwrap();
        let anon = Record {
            id: [1u8; 32],
            prev: [0u8; 32],
            actor_pubkey: [0u8; 32],
            actor_seq: 1,
            payload: b"eight!!!".to_vec(),
        };
        let before = st.pick().unwrap().arena_used;
        EvLog::append_bytes(&mut st, &anon).unwrap();
        let anon_cost = st.pick().unwrap().arena_used - before;

        let named = Record { actor_pubkey: [0xAA; 32], ..anon.clone() };
        let before = st.pick().unwrap().arena_used;
        EvLog::append_bytes(&mut st, &named).unwrap();
        let named_cost = st.pick().unwrap().arena_used - before;

        assert_eq!(named_cost - anon_cost, 4, "the key is four cells and only when it is there");
        let got = EvLog::walk(&st);
        assert_eq!(got[1].actor_pubkey, [0u8; 32], "an absent key reads back as zero");
        assert_eq!(got[0].actor_pubkey, [0xAA; 32], "a present one reads back whole");
    }

    /// THE TIP TRAVELS WITH THE RECORD. One commit, not two, and what the
    /// second one cost is measured here rather than assumed: a fresh root
    /// object, ten cells. The PartTab is NOT part of it -- `stage_commit`
    /// writes it into the fixed superblock region, not into the arena -- which
    /// is worth stating because the opposite is written down elsewhere.
    #[test]
    fn appending_with_the_tip_costs_one_commit_not_two() {
        let payload = b"a realistic little payload";
        let mut two = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut two).unwrap();
        let before = two.pick().unwrap().arena_used;
        EvLog::append_bytes(&mut two, &rec(1, payload)).unwrap();
        EvLog::set_tip_bytes(&mut two, &[1u8; 32]).unwrap();
        let two_cost = two.pick().unwrap().arena_used - before;

        let mut one = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut one).unwrap();
        let before = one.pick().unwrap().arena_used;
        EvLog::append_tip_bytes(&mut one, &rec(1, payload)).unwrap();
        let one_cost = one.pick().unwrap().arena_used - before;

        assert_eq!(EvLog::tip(&one), Some([1u8; 32]), "the tip is the record just written");
        assert_eq!(EvLog::walk(&one), EvLog::walk(&two), "the same log, written once");
        // The second commit was a whole root (8 + 2 cells) on top of the record.
        assert_eq!(two_cost - one_cost, 10, "one root's worth, saved");
    }

    /// A DAMAGED RECORD IS REPORTED, NOT PANICKED ON. A record whose flag
    /// claims an actor key that is not there used to read four cells past its
    /// own end; at the end of an arena that is an out-of-bounds index, which
    /// is a crash rather than a corrupt-image message.
    #[test]
    fn a_record_that_lies_about_its_actor_key_is_read_as_unnamed() {
        let mut st = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut st).unwrap();
        EvLog::append_tip_bytes(&mut st, &Record {
            id: [1u8; 32],
            prev: [0u8; 32],
            actor_pubkey: [0u8; 32], // unnamed: the record is 12 cells + payload
            actor_seq: 1,
            payload: b"x".to_vec(),
        })
        .unwrap();

        // Flip the flag bit on, leaving the record its unnamed length.
        let root = st.root().unwrap();
        let obj = st.follow(root, 1).unwrap();
        st.put_cell(obj, 11, FLAG_ACTOR);

        let got = EvLog::walk(&st);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].actor_pubkey, [0u8; 32], "a key that is not there reads as absent");
    }

    /// A v2 log says so in its root, and the say-so survives the byte trip that
    /// every hub image takes on every request.
    #[test]
    fn the_version_survives_the_byte_round_trip() {
        let mut st = Store::create_bytes(1 << 20);
        EvLog::init_bytes(&mut st).unwrap();
        EvLog::append_tip_bytes(&mut st, &rec(1, b"hello")).unwrap();
        let back = Store::from_bytes(&st.to_bytes_trimmed());
        assert_eq!(EvLog::version(&back), 2);
        assert_eq!(EvLog::walk(&back)[0].payload, b"hello");
        assert_eq!(EvLog::tip(&back), Some([1u8; 32]));
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
        // DERIVED FROM THE LAYOUT, not read off a run. In v2 a record is
        // 12 header cells, 4 more because `rec()` names an actor, and the
        // payload packed eight bytes to a cell -- ceil(21/8) = 3. Add the
        // store's 2-cell object header, and the new root: 8 cells and its own
        // 2-cell header.
        //
        // It was 47 in v1: 15 + 21 + 2 + 7 + 2, with one arena cell per payload
        // BYTE. The oracle here is the layout in this file's header; if the two
        // disagree, one of them is the bug.
        let payload_cells = (21usize.div_ceil(8)) as i64;
        assert_eq!(first, (12 + 4 + payload_cells + 2) + (8 + 2), "unexpected per-append cost");
        assert_eq!(first, 31, "and that is 31 cells, down from 47");
        let st = Store::open(p).unwrap();
        assert_eq!(EvLog::len(&st), 40);
        assert_eq!(EvLog::walk(&st).len(), 40);
        let _ = std::fs::remove_file(p);
    }

    /// Measure what a realistic 330-byte event actually costs in cells.
    /// The layout stores one payload byte per cell, so a 330-byte event needs
    /// 15 header + 330 payload + 2 object header = 347 cells per event,
    /// plus 7 + 2 root cells per commit = 356 cells total delta.
    /// The ratio (cells × 8) / payload_len gives the storage cost in bytes per byte.
    #[test]
    fn cells_per_event_for_a_realistic_payload() {
        let mut st = Store::create_bytes(8 << 20);
        EvLog::init_bytes(&mut st).unwrap();

        // Build a 330-byte payload
        let payload: Vec<u8> = (0..330).map(|i| (i % 256) as u8).collect();

        // Measure arena before
        let before = st.pick().unwrap().arena_used;

        // Append one record with the 330-byte payload
        EvLog::append_bytes(&mut st, &rec(1, &payload)).unwrap();

        // Measure arena after
        let after = st.pick().unwrap().arena_used;
        let delta = after - before;

        // v2: 12 header + 4 for the named actor + ceil(P/8) packed payload,
        // + 2 object header, + an 8-cell root and its 2-cell header.
        let expected = 12 + 4 + payload.len().div_ceil(8) as i64 + 2 + 8 + 2;
        assert_eq!(delta, expected, "cell delta should match layout formula");

        let ratio = (delta as f64 * 8.0) / payload.len() as f64;
        println!("Event with {} byte payload: {} cells used, ratio = {:.2} bytes/byte",
                 payload.len(), delta, ratio);
        // v1 stored one payload byte per cell and wrote the actor key whether
        // or not it was there: 356 cells for the same event, 8.6 bytes of
        // arena per byte of event.
        let v1 = 15 + payload.len() as i64 + 2 + 7 + 2;
        println!("v1 would have used {v1} cells; v2 uses {delta} ({}%)", delta * 100 / v1);
        assert!(delta * 5 < v1, "v2 must be far smaller: {delta} against {v1}");
    }

    /// Packing primitives: test round-trip of empty payload.
    #[test]
    fn pack_payload_empty_roundtrip() {
        let payload = b"";
        let packed = pack_payload(payload);
        assert_eq!(packed.len(), 0, "empty payload packs to zero cells");
        let unpacked = unpack_payload(&packed, 0);
        assert_eq!(unpacked, payload);
    }

    /// Test round-trip of 1-byte payload.
    #[test]
    fn pack_payload_one_byte_roundtrip() {
        let payload = b"A";
        let packed = pack_payload(payload);
        assert_eq!(packed.len(), 1, "1 byte packs into 1 cell");
        let unpacked = unpack_payload(&packed, payload.len());
        assert_eq!(unpacked, payload);
    }

    /// Test round-trip of 7-byte payload (one cell minus 1).
    #[test]
    fn pack_payload_seven_bytes_roundtrip() {
        let payload = b"abcdefg";
        let packed = pack_payload(payload);
        assert_eq!(packed.len(), 1, "7 bytes pack into 1 cell");
        let unpacked = unpack_payload(&packed, payload.len());
        assert_eq!(unpacked, payload);
    }

    /// Test round-trip of 8-byte payload (exactly one cell).
    #[test]
    fn pack_payload_eight_bytes_roundtrip() {
        let payload = b"12345678";
        let packed = pack_payload(payload);
        assert_eq!(packed.len(), 1, "8 bytes pack into 1 cell");
        let unpacked = unpack_payload(&packed, payload.len());
        assert_eq!(unpacked, payload);
    }

    /// Test round-trip of 9-byte payload (one cell plus 1).
    #[test]
    fn pack_payload_nine_bytes_roundtrip() {
        let payload = b"123456789";
        let packed = pack_payload(payload);
        assert_eq!(packed.len(), 2, "9 bytes pack into 2 cells");
        let unpacked = unpack_payload(&packed, payload.len());
        assert_eq!(unpacked, payload);
    }

    /// Test round-trip of 330-byte payload.
    #[test]
    fn pack_payload_330_bytes_roundtrip() {
        let payload: Vec<u8> = (0..330).map(|i| (i % 256) as u8).collect();
        let packed = pack_payload(&payload);
        // 330 bytes / 8 = 41 full cells + 2 extra bytes = 42 cells
        assert_eq!(packed.len(), 42, "330 bytes pack into exactly 42 cells");
        let unpacked = unpack_payload(&packed, payload.len());
        assert_eq!(unpacked, payload);
    }

    /// Verify the saving: old layout needs 330 cells for 330 bytes,
    /// new packing layout needs only 42 cells.
    #[test]
    fn pack_payload_saves_space_on_330_bytes() {
        let payload: Vec<u8> = (0..330).map(|i| (i % 256) as u8).collect();
        let packed = pack_payload(&payload);
        assert_eq!(payload.len(), 330, "payload is 330 bytes");
        assert_eq!(packed.len(), 42, "packed payload is 42 cells (vs 330 in v1)");
        // Verify round-trip
        let unpacked = unpack_payload(&packed, payload.len());
        assert_eq!(unpacked, payload);
    }

    /// Verify cell count formula for various sizes.
    #[test]
    fn pack_payload_cell_count_formula() {
        for len in [0, 1, 7, 8, 9, 15, 16, 17, 330] {
            let payload = vec![0u8; len];
            let packed = pack_payload(&payload);
            let expected_cells = (len + 7) / 8;
            assert_eq!(packed.len(), expected_cells,
                      "payload of {} bytes packs into {} cells", len, expected_cells);
        }
    }
}
