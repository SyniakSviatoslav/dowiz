//! COMMIT PROTOCOL v2 (R-EMBED-STORE §3.5; the order `docOps` in bebop-lang
//! formal/Bebop/Disk.lean proves atomic): which staged cells reach the device, in what order,
//! with which barriers.
//!
//! ```text
//! W arena[mark0, used_L)   every object every staged commit appended (each with its own crc)
//! W PartTab_L              21 cells at L.at + 16
//! B                        barrier 1: all the new superblock will name is durable
//! W superblock_L           16 cells at L.at, crc15 covers the anchor
//! B                        barrier 2: the commit point
//! ```
//!
//! GROUP FLUSH. The writer may stage N commits in memory and flush once. The superblocks
//! alternate pages, so the in-memory image holds the last two staged generations, L (live) and
//! O (the other page). If N is odd L sits opposite the durable superblock and is written as
//! above (O is on the durable page and is NOT written: writing it there would tear the only
//! durable generation). If N is even L sits ON the durable page; writing it first would put
//! the durable generation's page in a crash window, so this planner publishes O (generation
//! k+N-1, opposite) first as an inner commit, and only once it is durable does L overwrite the
//! old page:
//!
//! ```text
//! W arena[mark0, used_L) ; W PartTab_O ; B ; W superblock_O ; B ; W PartTab_L ; B ; W superblock_L ; B
//! ```
//!
//! A crash then recovers k, k+N-1 or k+N: a prefix of the group (AC-ES-18), never less than k.
//! W-ES-P11: `Writer::flush` never hands this planner an even group any more -- its FLIP moves
//! L to the page opposite the durable one in memory first (writer.rs `flip`), so every group
//! is the two-barrier shape above and recovers k or k+N (formal/Bebop/Group.lean). The inner
//! branch stays for a caller of `plan` that did not flip.

use super::IoError;
use crate::format::{PT_CURSOR, SB_GEN, SB_PARTTAB, SB_USED};
use crate::{Store, PARTTAB_CELLS, SB_B};

/// One step of a flush: write `n` cells of the staged image from `off`, or a barrier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Write { off: usize, n: usize },
    Barrier,
}

/// What the device holds as its live generation: the superblock's cell, its generation, and
/// the write cursor the next commit appends from (`begin`'s rule: the PartTab's cursor cell,
/// or the superblock's `arena_used` when there is no PartTab).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Durable {
    pub at: usize,
    pub gen: i64,
    pub cursor: i64,
}

impl Durable {
    /// The live generation of `st` as `pick` chooses it.
    pub fn of(st: &Store) -> Result<Durable, IoError> {
        let sb = st.pick().ok_or(IoError::Plan("the image has no live superblock"))?;
        Ok(Durable { at: sb.at, gen: sb.generation, cursor: cursor_of(st, sb.at) })
    }
}

/// `Store::begin`'s cursor for the superblock at `at` (0 when a cell is missing).
fn cursor_of(st: &Store, at: usize) -> i64 {
    let cell = |i: usize| st.cells.get(i).copied().unwrap_or(0);
    let pt = cell(at + SB_PARTTAB);
    if pt == 0 {
        cell(at + SB_USED)
    } else {
        cell(pt.max(0) as usize + PT_CURSOR)
    }
}

const PT_OBJ: usize = PARTTAB_CELLS as usize + 2;

/// The steps that take a device holding `durable` to the staged image `after`. Empty when
/// nothing was staged. Refuses an image that does not extend the durable one.
pub fn plan(durable: &Durable, after: &Store) -> Result<Vec<Step>, IoError> {
    let live = after.pick().ok_or(IoError::Plan("the staged image has no live superblock"))?;
    if live.generation == durable.gen && live.at == durable.at {
        return Ok(Vec::new());
    }
    if live.generation <= durable.gen {
        return Err(IoError::Plan("the staged generation is not newer than the durable one"));
    }
    let used = live.arena_used;
    if used < durable.cursor {
        return Err(IoError::Plan("the staged arena ends before the durable cursor"));
    }
    let other = SB_B - live.at;
    let other_gen = after.cells.get(other + SB_GEN).copied().unwrap_or(0);
    let inner = after.sb_valid(other) && other_gen > durable.gen && other != durable.at;
    if live.at == durable.at && !inner {
        return Err(IoError::Plan("the staged superblock would overwrite the durable one with no generation between"));
    }
    let mut steps = Vec::with_capacity(9);
    let mark0 = durable.cursor.max(0) as usize;
    if used as usize > mark0 {
        steps.push(Step::Write { off: mark0, n: used as usize - mark0 });
    }
    if inner {
        steps.extend(publish(other));
    }
    steps.extend(publish(live.at));
    Ok(steps)
}

/// PartTab, barrier, superblock, barrier: one generation published at `at`.
fn publish(at: usize) -> [Step; 4] {
    [
        Step::Write { off: at + crate::format::PT_PAGE_OFFSET, n: PT_OBJ },
        Step::Barrier,
        Step::Write { off: at, n: crate::format::SB_CELLS },
        Step::Barrier,
    ]
}

#[cfg(test)]
mod tests;
