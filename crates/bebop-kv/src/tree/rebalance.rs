//! The shape rules of a rewritten node (design §2.1), applied to its children after the ops: a
//! small child merges with a neighbour, an empty one is dropped, an overfull one splits evenly.
//!
//! W-ES-P8E: a child whose siblings ALL emptied and whose own slots fell under the minimum has no
//! neighbour here. Written as it stands it is a one-child chain the parent then links into a
//! sibling's node, REACHABLE, which `verify` refuses (`kv4del` at fanout 4, obj 5476, 2026-10-30).
//! Such a child is carried up UNWRITTEN as `Slot::Under` until a level with a neighbour, where it
//! is grafted into that neighbour's nearest edge path at its own level, rewriting the path.

use alloc::vec::Vec;

use super::node::node_at;
use super::write::{slots, split_even, Item, Key, WriteError, W};
use super::TreeError;

/// A child slot during a rebuild: untouched (kept by address), rewritten (its new slots, not
/// yet written), or under the minimum with no neighbour: `Under(its, level)` are the slots a
/// node at `level` would hold, to be merged into a cousin at that level.
pub(super) enum Slot<'o> {
    Old(Key<'o>, usize),
    New(Vec<Item<'o>>),
    Under(Vec<Item<'o>>, usize),
}

impl W<'_> {
    /// A slot's items and the level of the node they are the slots of (`level` unless `Under`).
    /// An old node read here is dead: it is about to be rewritten.
    fn open_slot<'o>(&mut self, s: Slot<'o>, level: usize) -> Result<(Vec<Item<'o>>, usize), WriteError> {
        match s {
            Slot::New(v) => Ok((v, level)),
            Slot::Under(v, lv) => Ok((v, lv)),
            Slot::Old(_, a) => {
                let nd = node_at(&*self.st, a, level == 1)?;
                self.kill(a);
                Ok((slots(self.st, &nd)?, level))
            }
        }
    }

    /// `src`, the slots of a node at `sl` (below `hl`), joined to the node at the end of `host`'s
    /// edge path (`right`: the rightmost, else the leftmost) at level `sl`. The path is rewritten
    /// bottom-up, split evenly where it overflows; the level-`hl` host's new slots come back.
    fn graft<'o>(&mut self, mut host: Vec<Item<'o>>, hl: usize, src: Vec<Item<'o>>, sl: usize, right: bool) -> Result<Vec<Item<'o>>, WriteError> {
        let e = if right { host.len().saturating_sub(1) } else { 0 };
        let Some(&(_, a)) = host.get(e) else {
            return Err(WriteError::Tree(TreeError::Root { why: "a graft into an empty node" }));
        };
        let below = hl - 1;
        let nd = node_at(&*self.st, a, below == 1)?;
        self.kill(a);
        let child = slots(self.st, &nd)?;
        let merged = if below == sl {
            let (mut l, r) = if right { (child, src) } else { (src, child) };
            l.extend(r);
            l
        } else {
            self.graft(child, below, src, sl, right)?
        };
        let mut pieces = Vec::new();
        for piece in split_even(merged, self.b) {
            let first = piece.first().map(|(k, _)| k.clone()).ok_or(WriteError::Tree(TreeError::Root { why: "an empty piece" }))?;
            pieces.push((first, self.write_node(&piece, below == 1)?));
        }
        host.splice(e..=e, pieces);
        Ok(host)
    }

    /// Two neighbouring slots at `level` as one: two nodes' slots concatenated, or an `Under`
    /// grafted into its neighbour at its own level. The result is `Under` when it is still below
    /// `level`, which only a higher level can resolve.
    fn merge<'o>(&mut self, left: Slot<'o>, right: Slot<'o>, level: usize) -> Result<Slot<'o>, WriteError> {
        let (l, ll) = self.open_slot(left, level)?;
        let (r, rl) = self.open_slot(right, level)?;
        let (its, lv) = match ll.cmp(&rl) {
            core::cmp::Ordering::Equal => {
                let mut v = l;
                v.extend(r);
                (v, ll)
            }
            core::cmp::Ordering::Greater => (self.graft(l, ll, r, rl, true)?, ll),
            core::cmp::Ordering::Less => (self.graft(r, rl, l, ll, false)?, rl),
        };
        Ok(if lv == level { Slot::New(its) } else { Slot::Under(its, lv) })
    }

    /// Merge each small or under slot into a neighbour, then write every rewritten child (split
    /// evenly when over the fanout). Returns the parent's new slots as `Slot::New`, or `Under`
    /// when the one slot left is too small to be a node of its own.
    pub(super) fn rebalance<'o>(&mut self, mut slots: Vec<Slot<'o>>, child_level: usize) -> Result<Slot<'o>, WriteError> {
        let leaf = child_level == 1;
        let min = (self.b / 4).max(2);
        slots.retain(|s| !matches!(s, Slot::New(v) if v.is_empty()));
        let mut i = 0;
        while i < slots.len() {
            let small = match slots.get(i) {
                Some(Slot::New(v)) => v.len() < min,
                Some(Slot::Under(..)) => true,
                _ => false,
            };
            if !small || slots.len() < 2 {
                i += 1;
                continue;
            }
            let a = i.saturating_sub(1);
            let right = slots.remove(a + 1);
            let left = slots.remove(a);
            let merged = self.merge(left, right, child_level)?;
            slots.insert(a, merged);
            i = a;
        }
        if slots.len() == 1 {
            match slots.pop() {
                Some(Slot::New(v)) if v.len() < min => return Ok(Slot::Under(v, child_level)),
                Some(Slot::Under(v, lv)) => return Ok(Slot::Under(v, lv)),
                Some(s) => slots.push(s),
                None => {}
            }
        }
        let mut out = Vec::with_capacity(slots.len());
        for s in slots {
            match s {
                Slot::Old(k, a) => out.push((k, a)),
                Slot::New(v) => {
                    for piece in split_even(v, self.b) {
                        let first = piece.first().map(|(k, _)| k.clone()).ok_or(WriteError::Tree(TreeError::Root { why: "an empty piece" }))?;
                        let a = self.write_node(&piece, leaf)?;
                        out.push((first, a));
                    }
                }
                // The loop above merges every `Under` while a neighbour exists.
                Slot::Under(..) => return Err(WriteError::Tree(TreeError::Root { why: "an under slot left beside a neighbour" })),
            }
        }
        Ok(Slot::New(out))
    }
}
