//! P8c / P13a: the COPY COMPACTION of a v4 image, and the trigger that asks for it.
//!
//! A tree commit kills the path it rewrites (`tx.sup_delta`), so a v4 image accumulates dead
//! cells at about the rate it writes them. Compaction is the bulk load (`stage_build`) of the live
//! entries into a fresh image whose first commit is generation g + 1 (never back to 1, as for
//! `compacted_from`). It runs ONLY when the dead cells pass the ratio (`dead_over`), when the
//! arena is full, or when the holder asks -- never on a commit count (v3's 32 deltas).
//!
//! HEADROOM (W-ES-P9): the fresh arena leaves `room_pct` percent of what the build used free
//! (`tree_compacted`: 100, as much again -- design P13a, arena <= ~2x live), rounded up to the
//! 16 KiB grid and capped at `max`. ONE build: the store starts at 16 KiB with its capacity
//! CLAIM at `max` (`alloc` grows the cells as it goes), and the capacity the image carries is
//! set from what the build used, before the commit that copies it into the PartTab and the
//! other superblock. Before this the arena was found by the v2 doubling walk -- which rebuilt
//! the whole tree once per attempt (three builds at 1e4, measured: the first always refused,
//! since its cap was under the values alone) and landed anywhere in [2x, 4x) of the used cells,
//! which `Store::from_bytes` then pads the writer's RAM to.

use alloc::vec::Vec;

use super::build::stage_build;
use super::write::WriteError;
use crate::{Store, StoreError};

/// The arena grid a fresh image's capacity lands on.
pub const CAP_GRID: usize = 16 * 1024;

/// Compact a v4 image when its superseded cells exceed its live cells / `dead_div` (`dead_div`
/// 1: the arena is at most about 2x what is live, design P13a). `dead_div <= 0` never triggers.
pub fn dead_over(st: &Store, dead_div: i64) -> bool {
    match st.pick() {
        Some(sb) if dead_div > 0 => sb.superseded_cells.saturating_mul(dead_div) > sb.live_cells,
        _ => false,
    }
}

/// The entries (strictly ascending) as a fresh v4 image, trimmed: generation `after_gen + 1`
/// when given, else 1; the arena the smallest 16 KiB multiple, up to `max` bytes, that holds
/// them with as much again free (or, at `max`, whatever fits).
pub fn tree_compacted<'e, I>(entries: I, max: usize, fanout: usize, after_gen: Option<i64>) -> Result<Vec<u8>, WriteError>
where
    I: Iterator<Item = (&'e [u8], &'e [u8])>,
{
    tree_compacted_room(entries, max, fanout, after_gen, 100)
}

/// `tree_compacted` with `room_pct` percent of the cells the build used left free (100 = as
/// much again; 0 = no room: the next commit compacts). The writer's RAM after the compaction is
/// the capacity (`Store::from_bytes` pads to it), so this is the memory lever as much as the
/// trigger lever.
pub fn tree_compacted_room<'e, I>(entries: I, max: usize, fanout: usize, after_gen: Option<i64>, room_pct: usize) -> Result<Vec<u8>, WriteError>
where
    I: Iterator<Item = (&'e [u8], &'e [u8])>,
{
    let mut st = Store::create_bytes(CAP_GRID.min(max))?;
    if let Some(g) = after_gen {
        crate::set_fresh_generation(&mut st, g)?;
    }
    // The claim `alloc` checks against: `max`. The cells grow as objects are written.
    set_capacity(&mut st, max)?;
    let (tx, root) = stage_build(&mut st, entries, fanout)?;
    // What the build used, in bytes from the image's start, plus the room asked for, on the grid.
    let used = tx.cursor.max(0) as usize * 8;
    let want = used.saturating_add(used.saturating_mul(room_pct) / 100);
    let cap = want.div_ceil(CAP_GRID).saturating_mul(CAP_GRID).min(max).max(used);
    set_capacity(&mut st, cap)?;
    st.commit_bytes(&tx, root);
    Ok(st.to_bytes_trimmed())
}

/// Superblock A of a fresh store (the one `begin` picked) says the image is `bytes` long: the
/// capacity cell and the crc over the first 15. `alloc` refuses past it; the commit carries it.
fn set_capacity(st: &mut Store, bytes: usize) -> Result<(), StoreError> {
    let cells = (bytes / 8).checked_sub(crate::ARENA).ok_or(StoreError::TooSmall { bytes, floor: crate::MIN_BYTES })?;
    let at = crate::SB_A + crate::format::SB_CAPACITY;
    match st.cells.get_mut(at) {
        Some(c) => *c = cells as i64,
        None => return Err(StoreError::Corrupt("a fresh store has no superblock")),
    }
    let crc = crate::crc32_cells(&st.cells, crate::SB_A, 15) as i64;
    match st.cells.get_mut(crate::SB_A + crate::format::SB_CRC) {
        Some(c) => *c = crc,
        None => return Err(StoreError::Corrupt("a fresh store has no superblock")),
    }
    Ok(())
}
