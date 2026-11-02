//! THE MERGE BODY (W-MG-M2, docs/design/M2-merge-design.md §2): what a device signs when it glues
//! two copies of one DOC. No new object and no format change: a MERGE is a record body under the
//! implementor's declaration framing, in the merging device's own chain, under that device's seal,
//! found through `Ids::body` like TRUNCATE and FORGOTTEN, and DOC stays root-version 5.
//!
//! ```text
//! MERGE  "ein-merge/1"
//!        ‖ n_ours LE8   ‖ (actor_id 32 ‖ seq LE8 ‖ tip 32) × n_ours     the HEADS it stood on
//!        ‖ n_theirs LE8 ‖ (actor_id 32 ‖ seq LE8 ‖ tip 32) × n_theirs   the HEADS it took in
//!        ‖ state u8                     2 Commutes, 3 Obstructed
//!        ‖ seg_root 32                  the newest shared SEG's root, 0^32 if the log was never cut
//!        ‖ dup_declared LE8 ‖ held_n LE8     0 and 0: M2-min refuses both cases (design §5.2, §5.3)
//!        ‖ n_conf LE8 ‖ (len LE8 ‖ bytes) × n_conf     each conflict, named by the implementor
//! ```
//!
//! EXACT, like FORGOTTEN: a body that carries the mark and is not exactly this is a MERGE that says
//! nothing, and it counts in `merges_bad`: an unparsable MERGE must not read as no MERGE at all.
//! State 1 (FastForward) is in the design's table and is never written: a fast-forward has one
//! history, so nothing is glued and no MERGE is signed. It parses, as a number with a place.
//! State 3 if and only if a conflict is named.
//!
//! WHAT `verify` JUDGES (`declare::merges_bad`): every row of an effective MERGE names a record of
//! this log at that `(actor, seq)`, or a seq under a cut of that actor. So a signed claim to have
//! seen a record the file does not hold is `merges_bad`, `holds` false, and a merged tail dropped
//! later with its seals (the rollback one file cannot see, `settle.rs`) is named as long as the
//! MERGE stays. Every other rule is the plain record's: id, dense seq, seal, budget.
//!
//! WHAT IT DOES NOT JUDGE. The conflict list is the signer's claim: this crate has no stock
//! semantics. The rows are what make it checkable: they fix both sides, so dowiz-hub re-derives
//! the conflicts from them (`doc_seal::merge`, `recheck`), and a list that does not follow from its
//! rows reads DIFFERS. One file cannot prove what the signer saw: the rows are signed claims too.

#[allow(unused_imports)]
use crate::prelude::*;
use super::seg::{id32, le8};

pub const MERGE_MARK: &[u8] = b"ein-merge/1";
pub const STATE_FAST_FORWARD: u8 = 1;
pub const STATE_COMMUTES: u8 = 2;
pub const STATE_OBSTRUCTED: u8 = 3;
const ROW_BYTES: usize = 72;

/// One device's position, as a HEADS row without its `ref SEAL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub actor_id: [u8; 32],
    pub seq: u64,
    pub tip: [u8; 32],
}

/// A MERGE body as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeView {
    pub ours: Vec<Row>,
    pub theirs: Vec<Row>,
    pub state: u8,
    pub seg_root: [u8; 32],
    pub conflicts: Vec<Vec<u8>>,
}

fn put_rows(b: &mut Vec<u8>, rows: &[Row]) {
    b.extend_from_slice(&(rows.len() as u64).to_le_bytes());
    for r in rows {
        b.extend_from_slice(&r.actor_id);
        b.extend_from_slice(&r.seq.to_le_bytes());
        b.extend_from_slice(&r.tip);
    }
}

/// The bytes of `v`; `state` is the caller's (`STATE_COMMUTES`, `STATE_OBSTRUCTED`).
pub fn merge_body(v: &MergeView) -> Vec<u8> {
    let mut b = Vec::with_capacity(MERGE_MARK.len() + 65 + (v.ours.len() + v.theirs.len()) * ROW_BYTES);
    b.extend_from_slice(MERGE_MARK);
    put_rows(&mut b, &v.ours);
    put_rows(&mut b, &v.theirs);
    b.push(v.state);
    b.extend_from_slice(&v.seg_root);
    b.extend_from_slice(&[0u8; 16]);
    b.extend_from_slice(&(v.conflicts.len() as u64).to_le_bytes());
    for c in &v.conflicts {
        b.extend_from_slice(&(c.len() as u64).to_le_bytes());
        b.extend_from_slice(c);
    }
    b
}

/// The rows behind a count at `at`, each handed to `row` with `which` (0 ours, 1 theirs): where they
/// end. The count is checked against what is left before one is read. `row` false stops the walk.
fn rows<R: FnMut(u8, Row) -> bool>(b: &[u8], mut at: usize, which: u8, row: &mut R) -> Option<usize> {
    let n = le8(b, at)?;
    at += 8;
    if n > (b.len().saturating_sub(at) / ROW_BYTES) as u64 {
        return None;
    }
    for _ in 0..n {
        if !row(which, Row { actor_id: id32(b, at)?, seq: le8(b, at + 32)?, tip: id32(b, at + 40)? }) {
            return None;
        }
        at += ROW_BYTES;
    }
    Some(at)
}

/// THE ONE WALK of a MERGE body: `row` per row, `conflict` per conflict, and the state and root.
/// `None` when `body` is not exactly a MERGE, or `row` said stop. No allocation of its own: `verify`
/// runs it, and every byte of it is the reader's wasm (a parse that collected cost 965 more).
pub(crate) fn walk_merge<R: FnMut(u8, Row) -> bool, C: FnMut(&[u8])>(body: &[u8], row: &mut R, conflict: &mut C) -> Option<(u8, [u8; 32])> {
    let b = body.strip_prefix(MERGE_MARK)?;
    let at = rows(b, 0, 0, row)?;
    let at = rows(b, at, 1, row)?;
    let state = *b.get(at)?;
    let seg_root = id32(b, at + 1)?;
    let n = le8(b, at + 49)?;
    if !(STATE_FAST_FORWARD..=STATE_OBSTRUCTED).contains(&state) || le8(b, at + 33)? != 0 || le8(b, at + 41)? != 0 || n > (b.len().saturating_sub(at + 57) / 8) as u64 {
        return None;
    }
    let mut at = at + 57;
    for _ in 0..n {
        let len = usize::try_from(le8(b, at)?).ok()?;
        let end = (at + 8).checked_add(len)?;
        conflict(b.get(at + 8..end)?);
        at = end;
    }
    (at == b.len() && (state == STATE_OBSTRUCTED) == (n > 0)).then_some((state, seg_root))
}

/// `None` when `body` is not exactly a MERGE (see the header).
pub fn parse_merge(body: &[u8]) -> Option<MergeView> {
    let (mut ours, mut theirs, mut conflicts) = (Vec::new(), Vec::new(), Vec::new());
    let (state, seg_root) = walk_merge(
        body,
        &mut |which, r| {
            if which == 0 { ours.push(r) } else { theirs.push(r) }
            true
        },
        &mut |c| conflicts.push(c.to_vec()),
    )?;
    Some(MergeView { ours, theirs, state, seg_root, conflicts })
}
