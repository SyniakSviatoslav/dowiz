//! The frontier rows of a segment chain as an index (W-MG-HOLES2). Rows: `seg.rs`.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use super::budget::set_of;
use super::seg::{Frontier, Segment};

/// Every frontier row, indexed once: `verify` asks per device and per seal, and a file can hold as
/// many rows as devices, so a scan per question is quadratic.
pub(crate) struct Frontiers {
    /// Per actor, its row in the newest segment that names it (`frontier_of`).
    newest: BTreeMap<[u8; 32], u64>,
    rows: Vec<Frontier>,
    /// Every tip of every row: where a `prev` walk may end.
    pub tips: BTreeSet<[u8; 32]>,
}

impl Frontiers {
    pub(crate) fn of(segs: &[Segment]) -> Self {
        let rows: Vec<Frontier> = segs.iter().flat_map(|s| s.frontier.iter().copied()).collect();
        let mut newest: BTreeMap<[u8; 32], u64> = BTreeMap::new();
        let mut i = 0u64;
        for f in &rows {
            newest.entry(f.actor_id).or_insert(i);
            i += 1;
        }
        let tips = set_of(rows.iter().map(|f| f.tip));
        Frontiers { newest, rows, tips }
    }
    pub(crate) fn of_actor(&self, actor: &[u8; 32]) -> Option<&Frontier> {
        self.newest.get(actor).and_then(|i| self.rows.get(*i as usize))
    }
    pub(crate) fn at(&self, actor: &[u8; 32], seq: u64) -> Option<&Frontier> {
        self.of_actor(actor).filter(|f| f.seq == seq)
    }
}
