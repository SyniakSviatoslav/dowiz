//! EVERY SEAL, READ ONCE (W-MG-HOLES2). `settle.rs` judges the seals many times, under different
//! REVOKEs; what a seal SAYS does not change between rounds. Walking HEADS each pass cost
//! heads × records × seal size (`atk2_dos`: two SEALs naming each other under every row), so the
//! walk happens here, once: a seal two rows reach is one seal, a ring ends where it closes and is
//! counted bad, and each object is paid for from the shared budget before it is copied.

#[allow(unused_imports)]
use crate::prelude::*;
use alloc::collections::BTreeMap;
use crate::Store;
use super::budget::{at_key, Budget};
use super::seal::{read_seal, Head, Seal};
use super::frontier::Frontiers;
use super::DocError;

/// One distinct seal.
pub(super) struct Item {
    pub seal: Seal,
    /// The tip its message names as the previous: the previous seal's, else the frontier row's
    /// (a device's oldest kept seal after a cut), else 0^32.
    pub prev_tip: [u8; 32],
    /// False when a HEADS row names it and disagrees: the row is signed by nobody, so it counts bad.
    pub agrees: bool,
    /// The last row whose walk came through it: seeing that row again is a ring.
    row: u64,
}

pub(super) struct Seals {
    pub all: Vec<Item>,
    /// Walks that ended on something that is no seal, or closed on themselves.
    pub bad: usize,
}

fn agrees(h: &Head, s: &Seal) -> bool {
    s.actor_id == h.actor_id && s.seq == h.seq && s.tip == h.tip
}

pub(super) fn load(st: &Store, heads: &[Head], fr: &Frontiers, budget: &mut Budget) -> Result<Seals, DocError> {
    let mut index: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    let (mut all, mut bad) = (Vec::<Item>::new(), 0usize);
    let mut row = 0u64;
    for h in heads {
        row += 1;
        let mut cur = h.seal;
        let mut first = true;
        while let Some(at) = cur {
            if let Some(it) = index.get(&at_key(at)).and_then(|i| all.get_mut(*i as usize)) {
                it.agrees &= !first || agrees(h, &it.seal);
                bad += usize::from(it.row == row);
                it.row = row;
                break;
            }
            budget.spend(st, at)?;
            let Some(seal) = st.check_obj(at).ok().and_then(|_| read_seal(st, at)) else {
                bad += 1;
                break;
            };
            index.insert(at_key(at), all.len() as u64);
            cur = seal.prev;
            all.push(Item { agrees: !first || agrees(h, &seal), seal, prev_tip: [0u8; 32], row });
            first = false;
        }
    }
    let tips: Vec<[u8; 32]> = all
        .iter()
        .map(|it| {
            let s = &it.seal;
            s.prev
                .and_then(|p| index.get(&at_key(p)))
                .and_then(|i| all.get(*i as usize))
                .map(|p| p.seal.tip)
                // The frontier row sits on a seal's edge (`CutInsideSeal`), and the oldest kept seal may
                // be a sync of many records, so its row is not always at `seq - 1`.
                .or_else(|| fr.of_actor(&s.actor_id).filter(|f| f.seq < s.seq).map(|f| f.tip))
                .unwrap_or([0u8; 32])
        })
        .collect();
    for (it, t) in all.iter_mut().zip(tips) {
        it.prev_tip = t;
    }
    Ok(Seals { all, bad })
}
