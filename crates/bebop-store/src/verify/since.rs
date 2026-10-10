//! THE LOG'S CRC, ONCE PER RECORD (W-HUBCRC, 2026-10-09).
//!
//! W-CRC made every append-log load hash EVERY record (`EvLog::chain_scan`): FRESH
//! 866-872 us for the 2 000-event bench log, so a Durable Object turn that loads the log,
//! appends one event and writes it back went 587 -> 1 339-1 968 us (R-LANG 2026-10-09
//! §3.2, W4). The log is APPEND-ONLY: an append writes one new record and a new root and
//! never touches an older record (`evlog::tests::..each_append_supersedes_exactly_the_old_root`),
//! so a holder that already scanned these bytes only needs the records added since.
//!
//! `ChainMark` is what a scan proved: the newest record it walked (cell index, its
//! crc/generation header cell, its 32-byte chain id) and how many records sat at or below
//! it. `chain_scan_since(st, Some(mark))` is `chain_scan` with ONE difference: once the
//! walk reaches the record the mark names -- same cell, same header cell, same id -- that
//! record and every older one are not hashed again. Everything else is unchanged and runs
//! on every load: the ROOT's crc, the whole walk with its `step_cap` and cell budget, the
//! count against the root's, and the crc + `prev`-link rule for every NEWER record.
//!
//! IT FALLS BACK, IT NEVER GUESSES. A mark whose record is not found (a grown or rebuilt
//! image lays records out elsewhere), whose id or header cell changed, or below which the
//! walk does not find exactly `below` records, gives the FULL scan -- the same answer
//! `chain_scan` gives, at its cost.
//!
//! WHAT A MARK TRUSTS, stated: that the cells at and below its record are the cells it
//! was taken on. Only a holder that keeps the bytes in its own memory may hold one -- the
//! DO drops its mark whenever bytes enter from storage or from a whole-image write
//! (`workers/api/src/hubdo/logmark.rs`), exactly as W-ZC drops its `Checked`. A cold load
//! passes no mark and hashes everything. `tests::a_mark_does_not_rehash_below_its_record`
//! shows the limit with one named cell.

use crate::evlog::EvLog;
use crate::verify::{BadCrc, ChainScan};
use crate::Store;

/// What a chain scan proved about the newest record it walked. Opaque; `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainMark {
    obj: usize,
    h1: i64,
    id: [i64; 4],
    below: usize,
}

impl ChainMark {
    fn of(st: &Store, obj: usize, below: usize) -> ChainMark {
        ChainMark { obj, h1: st.cell(obj + 1), id: id_cells(st, obj), below }
    }

    /// Is the record at `obj` the one this mark was taken on?
    fn names(&self, st: &Store, obj: usize) -> bool {
        obj == self.obj && st.cell(obj + 1) == self.h1 && id_cells(st, obj) == self.id
    }
}

/// A record's chain id: payload cells 3..6 (what `chain_scan`'s link rule compares).
fn id_cells(st: &Store, obj: usize) -> [i64; 4] {
    [st.get(obj, 3), st.get(obj, 4), st.get(obj, 5), st.get(obj, 6)]
}

impl EvLog {
    /// `chain_scan`, not re-hashing what `mark` covers (module doc). Returns the scan and the
    /// mark for THIS image (its newest record; `None` for an empty log or a chain that does
    /// not end). With a mark, `quarantined` lists only the records newer than it -- the
    /// older ones were judged when the mark was taken; `chained` is exactly `chain_scan`'s.
    pub fn chain_scan_since(st: &Store, mark: Option<&ChainMark>) -> Result<(ChainScan, Option<ChainMark>), BadCrc> {
        let (scan_out, next, held) = scan(st, mark)?;
        if held {
            return Ok((scan_out, next));
        }
        let (full, next, _) = scan(st, None)?;
        Ok((full, next))
    }
}

/// The walk. `held == false`: the mark's record was found but the count below it moved,
/// so what was skipped is not what the mark proved -- the caller re-runs without it.
fn scan(st: &Store, mark: Option<&ChainMark>) -> Result<(ChainScan, Option<ChainMark>, bool), BadCrc> {
    let Some(root) = st.root() else { return Ok((ChainScan { chained: Some(0), quarantined: Vec::new() }, None, true)) };
    st.check_obj(root)?;
    let cap = EvLog::step_cap(st);
    let mut budget = st.cells.len();
    let mut out = ChainScan { chained: None, quarantined: Vec::new() };
    let (mut n, mut newest, mut anchored_at) = (0usize, None, None);
    let mut cur = st.follow(root, 1);
    while let Some(obj) = cur {
        n += 1;
        let cost = st.obj_cells(obj).saturating_add(2);
        if n > cap || cost > budget {
            return Ok((out, None, true));
        }
        budget -= cost;
        newest.get_or_insert(obj);
        let next = st.follow(obj, 2);
        if anchored_at.is_none() && mark.is_some_and(|m| m.names(st, obj)) {
            anchored_at = Some(n);
        }
        if anchored_at.is_none() {
            if let Err(bad) = st.check_obj(obj) {
                if let Some(older) = next {
                    let linked = (0..4).all(|i| st.get(obj, 7 + i) == st.get(older, 3 + i));
                    if !linked {
                        return Err(bad);
                    }
                }
                out.quarantined.push(bad);
            }
        }
        cur = next;
    }
    if let (Some(m), Some(at)) = (mark, anchored_at) {
        if n + 1 - at != m.below {
            return Ok((out, None, false));
        }
    }
    out.chained = Some(n);
    Ok((out, newest.map(|o| ChainMark::of(st, o, n)), true))
}

#[cfg(test)]
mod tests;
