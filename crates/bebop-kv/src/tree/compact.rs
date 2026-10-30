//! P8c / P13a: the COPY COMPACTION of a v4 image, and the trigger that asks for it.
//!
//! A tree commit kills the path it rewrites (`tx.sup_delta`), so a v4 image accumulates dead
//! cells at about the rate it writes them. Compaction is the bulk load (`stage_build`) of the live
//! entries into a fresh image whose first commit is generation g + 1 (never back to 1, as for
//! `compacted_from`). It runs ONLY when the dead cells pass the ratio (`dead_over`), when the
//! arena is full, or when the holder asks -- never on a commit count (v3's 32 deltas).
//!
//! HEADROOM: v2's `compacted_from_fit` takes the SMALLEST doubling that holds the entries, so a
//! fresh image can be 99 % full and the next commit compacts again. A tree image is written to
//! afterwards, so this takes the smallest doubling that leaves at least as much free as used.

use alloc::vec::Vec;

use super::build::stage_build;
use super::write::WriteError;
use crate::{Store, StoreError};

/// Compact a v4 image when its superseded cells exceed its live cells / `dead_div` (`dead_div`
/// 1: the arena is at most about 2x what is live, design P13a). `dead_div <= 0` never triggers.
pub fn dead_over(st: &Store, dead_div: i64) -> bool {
    match st.pick() {
        Some(sb) if dead_div > 0 => sb.superseded_cells.saturating_mul(dead_div) > sb.live_cells,
        _ => false,
    }
}

/// The entries (strictly ascending) as a fresh v4 image, trimmed: generation `after_gen + 1`
/// when given, else 1; the arena the smallest doubling from 16 KiB, up to `max` bytes, that holds
/// them with as much again free (or, at `max`, whatever fits).
pub fn tree_compacted<'e, I>(entries: I, max: usize, fanout: usize, after_gen: Option<i64>) -> Result<Vec<u8>, WriteError>
where
    I: Iterator<Item = (&'e [u8], &'e [u8])> + Clone,
{
    let mut cap = 16 * 1024;
    loop {
        let mut st = Store::create_bytes(cap)?;
        if let Some(g) = after_gen {
            crate::set_fresh_generation(&mut st, g)?;
        }
        match stage_build(&mut st, entries.clone(), fanout) {
            Ok((tx, root)) => {
                st.commit_bytes(&tx, root);
                let used = st.pick().map_or(0, |sb| sb.arena_used.max(0) as usize) * 8;
                if used * 2 > cap && cap < max {
                    cap = (cap * 2).min(max);
                    continue;
                }
                return Ok(st.to_bytes_trimmed());
            }
            Err(WriteError::Store(StoreError::ArenaFull { .. })) if cap < max => cap = (cap * 2).min(max),
            Err(e) => return Err(e),
        }
    }
}
