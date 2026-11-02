//! `recover` -- what `open` runs (R-EMBED-STORE §3.6), over `Cells`, writing nothing:
//!
//! ```text
//! pick (format::pick_checked_in): valid superblocks, higher generation, every offset checked
//! anchor Holds                 -> OK, O(last commit)
//! anchor Absent                -> walk the arena from the OTHER valid superblock's arena_used
//!                                 (bebop's st_reopen_lo rule) or from 1024, to arena_used
//! anchor Torn, or walk fails   -> hide this superblock IN MEMORY, pick again, walk the
//!                                 fallback's WHOLE arena (st_reopen_verify's rule)
//! both fail                    -> RecoverError::Both
//! ```
//!
//! A STRUCTURAL refusal of the newer superblock (`PickError` other than "none valid") stays a
//! refusal: FORMAT §3.1 item 3 allows a fallback only past a bad magic/crc15, and a newer
//! valid superblock that does not fit is a truncation, never a reason to read an older
//! generation. The fallback added here is past a superblock whose crc holds but whose LAST
//! COMMIT does not -- the case bebop's `st_reopen_verify` heals by zeroing the magic in its
//! MAP_SHARED mapping. A library reader must not write (§3.6), so the hiding is in memory
//! (`Hide`) and `Recovered::hidden` names the cell; the writer repairs it on the device.

use crate::format::{anchor_in, pick_checked_in, Anchor, PickError, SB_USED};
use crate::{obj_crc_ok_in, obj_len_in, sb_valid_in, Cells, Superblock, ARENA, MAGIC, SB_B};

/// How the picked generation was proved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Via {
    /// The anchor held over `cells` arena cells (0 objects walked: AC-ES-17's Rust half).
    Anchor { cells: usize },
    /// No anchor: `objects` objects walked from `from`.
    Walk { from: usize, objects: usize },
    /// The newer superblock was refused for `why`; the older one's whole arena walked.
    Fallback { from_gen: i64, why: Why, objects: usize },
}

/// Why a picked generation was not accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    Pick(PickError),
    Anchor(Anchor),
    /// The object at this cell is cut short or fails its crc.
    Walk { at: usize },
}

/// The generation recovery opens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recovered {
    pub sb: Superblock,
    pub via: Via,
    /// The superblock (its magic cell) a reader must treat as invalid, if any.
    pub hidden: Option<usize>,
}

/// Why nothing could be opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoverError {
    /// The pick itself refused (no valid superblock, or a structural refusal).
    Pick(PickError),
    /// The newer generation's last commit failed, and so did the fallback.
    Both { live: Why, other: Why },
}

/// `Cells` with one superblock's magic read as 0: that superblock is invalid to every rule.
pub struct Hide<'a, C: Cells + ?Sized> {
    pub inner: &'a C,
    pub at: usize,
}

impl<C: Cells + ?Sized> Cells for Hide<'_, C> {
    fn cell_at(&self, i: usize) -> i64 {
        if i == self.at { 0 } else { self.inner.cell_at(i) }
    }
    fn n_cells(&self) -> usize {
        self.inner.n_cells()
    }
}

/// Walk the objects from `from` to `to`: each must fit before `to` and pass its crc.
/// Returns how many there were, or the cell of the first that did not.
pub fn walk<C: Cells + ?Sized>(c: &C, from: usize, to: usize) -> Result<usize, usize> {
    let (mut i, mut n) = (from, 0usize);
    while i < to {
        let len = obj_len_in(c, i) as usize;
        let end = i.saturating_add(2).saturating_add(len);
        if end > to || !obj_crc_ok_in(c, i) {
            return Err(i);
        }
        i = end;
        n += 1;
    }
    Ok(n)
}

/// The arena range a commit could have left inconsistent: from the other valid superblock's
/// `arena_used` (everything below it was durable before this generation's window opened) or
/// from `ARENA`.
fn walk_from<C: Cells + ?Sized>(c: &C, sb: &Superblock) -> usize {
    let o = SB_B - sb.at;
    if sb_valid_in(c, o) {
        let u = c.cell_at(o + SB_USED);
        if u >= ARENA as i64 && u <= sb.arena_used {
            return u as usize;
        }
    }
    ARENA
}

/// Prove the picked generation's last commit.
fn check_last<C: Cells + ?Sized>(c: &C, sb: &Superblock) -> Result<Via, Why> {
    match anchor_in(c, sb) {
        Anchor::Holds { mark, used } => Ok(Via::Anchor { cells: used - mark }),
        t @ Anchor::Torn { .. } => Err(Why::Anchor(t)),
        Anchor::Absent => {
            let from = walk_from(c, sb);
            let objects = walk(c, from, sb.arena_used.max(0) as usize).map_err(|at| Why::Walk { at })?;
            Ok(Via::Walk { from, objects })
        }
    }
}

/// §3.6, over any image. Writes nothing.
pub fn recover<C: Cells + ?Sized>(c: &C) -> Result<Recovered, RecoverError> {
    let sb = pick_checked_in(c).map_err(RecoverError::Pick)?;
    let live = match check_last(c, &sb) {
        Ok(via) => return Ok(Recovered { sb, via, hidden: None }),
        Err(why) => why,
    };
    debug_assert_eq!(c.cell_at(sb.at), MAGIC);
    let hidden = Hide { inner: c, at: sb.at };
    let o = pick_checked_in(&hidden).map_err(|e| RecoverError::Both { live, other: Why::Pick(e) })?;
    let objects = walk(&hidden, ARENA, o.arena_used.max(0) as usize)
        .map_err(|at| RecoverError::Both { live, other: Why::Walk { at } })?;
    Ok(Recovered { sb: o, via: Via::Fallback { from_gen: sb.generation, why: live, objects }, hidden: Some(sb.at) })
}

#[cfg(test)]
mod tests;
