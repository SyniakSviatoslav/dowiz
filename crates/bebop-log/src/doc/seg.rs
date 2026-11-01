//! SEG, and the two record bodies that keep a cut and an erasure honest (W-MG-M3/M4,
//! R-MAINGOAL-A §1.2, §2b, §2c). The log is KEPT HISTORY: records leave the file only as a
//! whole segment, under a root, and only by a TRUNCATE the cutting device signs through its
//! seal. Nothing else removes a record; a redaction (`forget.rs`'s rule) keeps it in place.
//!
//! ```text
//! SEG v1  {VERSION=1, first, last, root 4, ref PREV_SEG, ref BLOB, redacted, declared,
//!          m, (actor_id 4, seq, tip 4, scheme) × m}                       12 + 10m cells
//! ```
//!
//! `scheme` (W-MG-M3b) is M0's forward-once rule carried across the cut: 1 when a CHAINED
//! (`Ids::chained`) record of the device had been seen by the cut, 0 while every record of
//! its was legacy. `verify` starts the device's `moved` from it, so a legacy id just after a
//! cut of chained records is broken, as it would have been in the uncut file. (The row was 9
//! cells, digest 3288852188, until 2026-10-31; no production image holds a SEG, so the layout
//! moved without a version.)
//!
//! `first`/`last` are the cut records' FILE positions, counted from the first record the
//! file ever held (the segments chain back through `ref PREV_SEG`, oldest last; a file's
//! live log starts at the newest segment's `last + 1`). `root` is the implementor's tree
//! (`Ids::tree_root`: RFC 9162 over the ids in file order) -- the one value a seal, a
//! witness or an archive cites for the whole range. `BLOB` is 0 here: the cut records live
//! in the archive the cut handed back, exactly as `rotate.rs` keeps an archive today; a
//! packed copy inside the file is P15 S4's (R-BINREC §8.5), not this phase's. `redacted` /
//! `declared` are the tombstones and the FORGOTTEN sums under the cut, so law 9 (redacted ==
//! declared) still balances when a declaration and its tombstones part ways at a cut. The
//! rows are the FRONTIER at the cut, one per device: the `(seq, tip)` of its newest cut
//! record, which is the `prev_tip` its oldest kept seal signed (`seal.rs`) -- a seal with no
//! `ref PREV_SEAL` reads it here.
//!
//! THE SEG HASH (W-MG-HOLES). A seal citing a row proved nothing: a moved or added row excused a
//! hole in the kept log, and the counts of a segment below the newest were nobody's claim, though
//! `verify` sums them (`atk_holes` holes 5, 6). So `chain_hash` runs over EVERY cell of every
//! segment that means anything, rows and counts included, oldest first, each hash over the one
//! below it, and the TRUNCATE signs the newest. One signature on the newest covers the chain; a
//! later cut may take the older TRUNCATE records. What it does NOT cover: the records that left.
//! `root` names their ids, and a tombstone keeps its id, so a tombstone's bytes under a cut are
//! bound only by a FORGOTTEN still in the file; the archive is what shows them.
//!
//! The bodies, found through `Ids::body` like `keys.rs`'s:
//!
//! ```text
//! TRUNCATE   "bebop-truncate/1" ‖ root 32 ‖ prev_root 32 ‖ first LE8 ‖ last LE8 ‖ redacted LE8 ‖ declared LE8 ‖ seg_hash 32
//! FORGOTTEN  "bebop-forgotten/1" ‖ n_redacted LE8 ‖ subject 32 ‖ (id 32 ‖ digest 32) × n_redacted
//! ```
//!
//! A TRUNCATE is EFFECTIVE when its record chains, its actor is a ROOT and an ok seal of that actor
//! covers its seq (`declare.rs`). A FORGOTTEN is effective with any admitted actor;
//! `verify` sums its `n_redacted` as `declared`. It LISTS what it covers: each tombstone's id and
//! its `digest`, `Ids::chained` over the tombstone as it now stands (flags, actor, seq, `prev`,
//! tombstone payload), because the id itself stopped checking those. A tombstone is a mark
//! anyone can set, so a count alone balanced whichever record a forger erased (`atk_holes` hole 3).
//! Why the declaration is signed and the nonce erased with the fields: §2c -- an erased id must
//! confirm no guess (`dowiz-hub/src/forget/tests.rs` enumerates ten thousand phones).

#[allow(unused_imports)]
use crate::prelude::*;
use crate::{Store, StoreError, Tx};
use super::budget::Budget;
use super::{cells_of, cells_to_id, id_to_cells, DocError, Ids};
pub use super::forgotten::{forgotten_body, parse_forgotten, Erased, FORGOTTEN_MARK};

pub const SEG_VERSION: i64 = 1;
pub const LAYOUT_SEG: &str = "SEG{i64,i64,i64,i64,i64,i64,i64,ref SEG,ref BLOB,i64,i64,i64,[i64,i64,i64,i64,i64,i64,i64,i64,i64,i64]}";
/// `st_digest(LAYOUT_SEG)`; 3288852188 was the 9-cell row without `scheme` (`dsa_tests.rs`
/// keeps it as the control). Mirrored by `tools/verify` (python) -- tell its owner on a move.
pub const DIGEST_SEG: i64 = 2192499951;
/// The tag in front of `seg_message`.
pub const SEG_TAG: &[u8] = b"bebop-seg/1";
pub const TRUNCATE_MARK: &[u8] = b"bebop-truncate/1";
const SEG_HEAD: usize = 12;
const SEG_ROW: usize = 10;
/// The `scheme` cell of a frontier row: no chained record of the device yet ...
pub const SCHEME_LEGACY: i64 = 0;
/// ... or the device has moved to `Ids::chained` (M0: once, forward, never back).
pub const SCHEME_CHAINED: i64 = 1;

/// One device's position at a cut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frontier {
    pub actor_id: [u8; 32],
    pub seq: u64,
    pub tip: [u8; 32],
    /// `SCHEME_CHAINED` once a chained record of the device left under a cut.
    pub scheme: i64,
}

/// One segment as read (or as `cut.rs` is about to write it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub first: u64,
    pub last: u64,
    pub root: [u8; 32],
    /// The segment before this one (absolute offset), and ITS root (0^32 for the first).
    pub prev: Option<usize>,
    pub prev_root: [u8; 32],
    pub blob: Option<usize>,
    pub redacted: u64,
    pub declared: u64,
    pub frontier: Vec<Frontier>,
    /// `chain_hash` with this segment on top. Only `Doc::cut_into` fills it, for
    /// `truncate_body`; a segment read back holds 0^32 and `verify` recomputes the chain.
    pub seg_hash: [u8; 32],
}

/// What one segment's hash covers, `below` being the hash of the segment under it (0^32 for the
/// oldest): every field a reader acts on. `blob` as a bit: BLOB is 0 and its offset is not a fact.
pub fn seg_message(s: &Segment, below: &[u8; 32]) -> Vec<u8> {
    let mut m = Vec::with_capacity(SEG_TAG.len() + 121 + s.frontier.len() * 80);
    m.extend_from_slice(SEG_TAG);
    m.extend_from_slice(below);
    m.extend_from_slice(&s.root);
    m.push(s.blob.is_some() as u8);
    for n in [s.first, s.last, s.redacted, s.declared, s.frontier.len() as u64] {
        m.extend_from_slice(&n.to_le_bytes());
    }
    for f in &s.frontier {
        m.extend_from_slice(&f.actor_id);
        m.extend_from_slice(&f.seq.to_le_bytes());
        m.extend_from_slice(&f.tip);
        m.extend_from_slice(&f.scheme.to_le_bytes());
    }
    m
}

/// The hash of a segment chain (`segs` newest first, as `read_segs` gives it). `None` when `ids`
/// has no hash.
pub fn chain_hash(segs: &[Segment], ids: &dyn Ids) -> Option<[u8; 32]> {
    segs.iter().rev().try_fold([0u8; 32], |below, s| ids.hash(&seg_message(s, &below)))
}

impl Segment {
    pub fn count(&self) -> u64 {
        self.last.saturating_sub(self.first).saturating_add(1)
    }

    /// The TRUNCATE body that declares this segment.
    pub fn truncate_body(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(TRUNCATE_MARK.len() + 64 + 32);
        b.extend_from_slice(TRUNCATE_MARK);
        b.extend_from_slice(&self.root);
        b.extend_from_slice(&self.prev_root);
        for n in [self.first, self.last, self.redacted, self.declared] {
            b.extend_from_slice(&n.to_le_bytes());
        }
        b.extend_from_slice(&self.seg_hash);
        b
    }

    /// Does a TRUNCATE body's six fields name exactly this segment?
    pub fn is_named_by(&self, t: &Truncate) -> bool {
        t.root == self.root && t.prev_root == self.prev_root && t.first == self.first && t.last == self.last && t.redacted == self.redacted && t.declared == self.declared
    }
}

/// A parsed TRUNCATE body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Truncate {
    pub root: [u8; 32],
    pub prev_root: [u8; 32],
    pub first: u64,
    pub last: u64,
    pub redacted: u64,
    pub declared: u64,
    pub seg_hash: [u8; 32],
}

pub(super) fn le8(b: &[u8], at: usize) -> Option<u64> {
    b.get(at..at + 8).and_then(|w| w.try_into().ok()).map(u64::from_le_bytes)
}
pub(super) fn id32(b: &[u8], at: usize) -> Option<[u8; 32]> {
    b.get(at..at + 32).and_then(|w| w.try_into().ok())
}

/// `None` when the body is not a TRUNCATE.
pub fn parse_truncate(body: &[u8]) -> Option<Truncate> {
    let r = body.strip_prefix(TRUNCATE_MARK)?;
    Some(Truncate {
        root: id32(r, 0)?,
        prev_root: id32(r, 32)?,
        first: le8(r, 64)?,
        last: le8(r, 72)?,
        redacted: le8(r, 80)?,
        declared: le8(r, 88)?,
        seg_hash: id32(r, 96)?,
    })
}

/// One SEG object; `None` for a wrong digest, version or length.
pub fn read_seg(st: &Store, at: usize) -> Option<Segment> {
    if st.obj_digest(at) != DIGEST_SEG || st.obj_cells(at) < SEG_HEAD || st.get(at, 0) != SEG_VERSION {
        return None;
    }
    // Against the object and in the cell's own width: `m * SEG_ROW` wraps on wasm32 at 2^31 rows.
    let rows = st.obj_cells(at) - SEG_HEAD;
    if rows % SEG_ROW != 0 || (rows / SEG_ROW) as i64 != st.get(at, 11).max(0) {
        return None;
    }
    let m = rows / SEG_ROW;
    // Compared as the u64s the counters read them as: a segment is at least one record.
    if (st.get(at, 2) as u64) < (st.get(at, 1) as u64) {
        return None;
    }
    let prev = st.follow(at, 7);
    let prev_root = prev.and_then(|p| read_seg_root(st, p)).unwrap_or([0u8; 32]);
    let frontier = (0..m)
        .map(|i| {
            let b = SEG_HEAD + i * SEG_ROW;
            Frontier { actor_id: cells_to_id(&cells_of(st, at, b, 4)), seq: st.get(at, b + 4) as u64, tip: cells_to_id(&cells_of(st, at, b + 5, 4)), scheme: st.get(at, b + 9) }
        })
        .collect();
    Some(Segment {
        first: st.get(at, 1) as u64,
        last: st.get(at, 2) as u64,
        root: cells_to_id(&cells_of(st, at, 3, 4)),
        prev,
        prev_root,
        blob: st.follow(at, 8),
        redacted: st.get(at, 9) as u64,
        declared: st.get(at, 10) as u64,
        frontier,
        seg_hash: [0u8; 32],
    })
}

/// Only the root of the SEG at `at` (what a `prev_root` is read from), bounded like `read_seg`.
fn read_seg_root(st: &Store, at: usize) -> Option<[u8; 32]> {
    (st.obj_digest(at) == DIGEST_SEG && st.obj_cells(at) >= SEG_HEAD && st.get(at, 0) == SEG_VERSION).then(|| cells_to_id(&cells_of(st, at, 3, 4)))
}

/// Every segment from the newest (`newest` = the DOC's `ref SEG`) back, newest first, each
/// crc checked: a damaged or foreign object on the chain is a refusal, never a shorter
/// chain (a shorter chain would be fewer segments to declare). `cap` bounds a chain that
/// does not end.
pub fn read_segs(st: &Store, newest: Option<usize>, cap: usize) -> Result<Vec<Segment>, DocError> {
    read_segs_in(st, newest, cap, &mut Budget::of(st))
}

/// `read_segs` paying from `budget`, before each read.
pub(crate) fn read_segs_in(st: &Store, newest: Option<usize>, cap: usize, budget: &mut Budget) -> Result<Vec<Segment>, DocError> {
    let mut out = Vec::new();
    let mut cur = newest;
    while let Some(at) = cur {
        if out.len() >= cap {
            return Err(DocError::Corrupt("the SEG chain never ends"));
        }
        budget.spend(st, at)?;
        st.check_obj(at)?;
        let s = read_seg(st, at).ok_or(DocError::Corrupt("a ref SEG is no SEG"))?;
        cur = s.prev;
        out.push(s);
    }
    Ok(out)
}

/// `actor`'s row in the newest segment (`segs` newest first) that names it: where the
/// device stood when its records last left the file.
pub fn frontier_of(segs: &[Segment], actor: &[u8; 32]) -> Option<Frontier> {
    segs.iter().find_map(|s| s.frontier.iter().find(|f| f.actor_id == *actor).copied())
}

pub(crate) fn stage_seg(st: &mut Store, tx: &mut Tx, s: &Segment) -> Result<usize, StoreError> {
    let o = st.alloc(tx, (SEG_HEAD + s.frontier.len() * SEG_ROW) as i64, DIGEST_SEG)?;
    st.put_cell(o, 0, SEG_VERSION);
    st.put_cell(o, 1, s.first as i64);
    st.put_cell(o, 2, s.last as i64);
    for (i, c) in id_to_cells(&s.root).iter().enumerate() {
        st.put_cell(o, 3 + i, *c);
    }
    match s.prev {
        Some(p) => st.link(o, 7, p),
        None => st.put_cell(o, 7, 0),
    }
    match s.blob {
        Some(b) => st.link(o, 8, b),
        None => st.put_cell(o, 8, 0),
    }
    st.put_cell(o, 9, s.redacted as i64);
    st.put_cell(o, 10, s.declared as i64);
    st.put_cell(o, 11, s.frontier.len() as i64);
    for (i, f) in s.frontier.iter().enumerate() {
        let b = SEG_HEAD + i * SEG_ROW;
        for (j, c) in id_to_cells(&f.actor_id).iter().enumerate() {
            st.put_cell(o, b + j, *c);
        }
        st.put_cell(o, b + 4, f.seq as i64);
        for (j, c) in id_to_cells(&f.tip).iter().enumerate() {
            st.put_cell(o, b + 5 + j, *c);
        }
        st.put_cell(o, b + 9, f.scheme);
    }
    st.seal(o);
    Ok(o)
}
