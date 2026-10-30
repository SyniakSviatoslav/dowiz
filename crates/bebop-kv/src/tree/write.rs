//! The v4 WRITER: one commit = one batched PATH COPY (design §2.1).
//!
//! The batch is sorted, one op per key (`Batch::ops`). Each node with an op under it is read once
//! and written once per commit, however many ops hit it; a child without ops keeps its OLD address,
//! so the new node links to it -- shared, not copied. Allocation is post-order (values, then
//! leaves, then nodes, then the root last), so every ref is strictly backward by construction.
//!
//! Shape rules: a node over the fanout splits into `ceil(m/B)` EVEN pieces; a rewritten node under
//! `max(B/4, 2)` merges with a neighbour (which is rewritten too); an empty one is dropped; the root
//! collapses while it has one child and grows a level while it has more than B.

use alloc::vec::Vec;

use super::node::{items, node_at, ref_at};
use super::{TreeError, DIGEST_KV4_LEAF, DIGEST_KV4_NODE, DIGEST_KV4_VAL, KIND_LEAF, KIND_NODE, MAX_FANOUT, MAX_TREE_DEPTH, MIN_FANOUT, ROOT_V4, VERSION_TREE};
use crate::delta::KeyOp;
use crate::{Store, StoreError, Tx, DIGEST_KV_ROOT};

/// Why a tree commit was not staged. A refusal before `begin` leaves the image untouched.
#[derive(Debug)]
pub enum WriteError {
    Store(StoreError),
    /// The tree this commit starts from is not readable.
    Tree(TreeError),
    /// The image's KV is version `v` (1-3): those are written by `append_delta` / compaction, and
    /// move to v4 only through `migrate_to_tree`.
    NotTree { v: i64 },
    /// The ops are not strictly ascending by key (one op per key, as `Batch::ops` gives them).
    Unsorted,
    /// A fanout outside `MIN_FANOUT..=MAX_FANOUT`.
    Fanout(usize),
    /// `stage_build` into a store that already has a root.
    NotFresh,
    /// `migrate_to_tree`'s source was refused.
    Kv(crate::verify::KvError),
}

impl From<StoreError> for WriteError {
    fn from(e: StoreError) -> WriteError {
        WriteError::Store(e)
    }
}
impl From<TreeError> for WriteError {
    fn from(e: TreeError) -> WriteError {
        WriteError::Tree(e)
    }
}

/// (first key, absolute address): one slot of a node about to be written.
pub(crate) type Item = (Vec<u8>, usize);

/// A child slot during a rebuild: untouched (kept by address) or rewritten (its new items).
enum Slot {
    Old(Vec<u8>, usize),
    New(Vec<Item>),
}

pub(crate) struct W<'s> {
    pub st: &'s mut Store,
    pub tx: Tx,
    pub b: usize,
    /// Cells this commit makes dead (old path, old values, old root), for `sup_delta`.
    pub dead: i64,
    /// Entries added minus entries removed.
    pub dn: i64,
}

pub(crate) fn key_of<'a>(op: &KeyOp<'a>) -> &'a [u8] {
    match *op {
        KeyOp::Put(k, _) | KeyOp::Remove(k) => k,
    }
}

/// Write `bytes` packed eight to a cell from payload cell `at` of `obj` (the last cell zero-filled).
fn write_packed(st: &mut Store, obj: usize, at: usize, bytes: &[u8]) {
    for (c, chunk) in bytes.chunks(8).enumerate() {
        let mut w = [0u8; 8];
        if let Some(d) = w.get_mut(..chunk.len()) {
            d.copy_from_slice(chunk);
        }
        st.put_cell(obj, at + c, i64::from_le_bytes(w));
    }
}

/// `v` cut into `ceil(len/b)` pieces whose sizes differ by at most one.
pub(crate) fn split_even(v: Vec<Item>, b: usize) -> Vec<Vec<Item>> {
    let k = v.len().div_ceil(b).max(1);
    let (base, rem) = (v.len() / k, v.len() % k);
    let mut it = v.into_iter();
    (0..k).map(|p| it.by_ref().take(base + usize::from(p < rem)).collect()).collect()
}

impl W<'_> {
    pub fn write_val(&mut self, v: &[u8]) -> Result<usize, WriteError> {
        let obj = self.st.alloc(&mut self.tx, 1 + v.len().div_ceil(8) as i64, DIGEST_KV4_VAL)?;
        self.st.put_cell(obj, 0, v.len() as i64);
        write_packed(self.st, obj, 1, v);
        self.st.seal(obj);
        Ok(obj)
    }

    /// One node from its slots (refs to objects already written: backward).
    pub fn write_node(&mut self, its: &[Item], leaf: bool) -> Result<usize, WriteError> {
        let m = its.len();
        let keys: Vec<u8> = its.iter().flat_map(|(k, _)| k.iter().copied()).collect();
        let len = 1 + 2 * m + keys.len().div_ceil(8);
        let (dig, kind) = if leaf { (DIGEST_KV4_LEAF, KIND_LEAF) } else { (DIGEST_KV4_NODE, KIND_NODE) };
        let obj = self.st.alloc(&mut self.tx, len as i64, dig)?;
        self.st.put_cell(obj, 0, ((kind << 56) | m as u64) as i64);
        let mut koff = 0usize;
        for (i, (k, a)) in its.iter().enumerate() {
            let kc = ((koff as i64) << 16) | k.len() as i64;
            let (kcell, rcell) = if leaf { (1 + 2 * i, 2 + 2 * i) } else { (2 + 2 * i, 1 + 2 * i) };
            self.st.put_cell(obj, kcell, kc);
            self.st.link(obj, rcell, *a);
            koff += k.len();
        }
        write_packed(self.st, obj, 1 + 2 * m, &keys);
        self.st.seal(obj);
        Ok(obj)
    }

    pub fn kill(&mut self, obj: usize) {
        self.dead += 2 + self.st.obj_cells(obj) as i64;
    }

    /// A leaf's entries with the ops merged in: a put writes its value and overwrites or inserts,
    /// a remove deletes (a remove of an absent key changes nothing).
    pub fn leaf_merge(&mut self, old: Vec<Item>, ops: &[KeyOp]) -> Result<Vec<Item>, WriteError> {
        let mut out = Vec::with_capacity(old.len() + ops.len());
        let mut it = old.into_iter().peekable();
        for op in ops {
            let k = key_of(op);
            while let Some(e) = it.next_if(|(ok, _)| ok.as_slice() < k) {
                out.push(e);
            }
            let hit = it.next_if(|(ok, _)| ok.as_slice() == k);
            if let Some((_, v)) = &hit {
                self.kill(*v);
            }
            match op {
                KeyOp::Put(_, v) => {
                    let a = self.write_val(v)?;
                    out.push((k.to_vec(), a));
                    self.dn += i64::from(hit.is_none());
                }
                KeyOp::Remove(_) => self.dn -= i64::from(hit.is_some()),
            }
        }
        out.extend(it);
        Ok(out)
    }

    /// The new slots of the node at `obj` (`level` 1 = leaf) with `ops` applied, NOT yet written.
    pub fn apply(&mut self, obj: usize, level: usize, ops: &[KeyOp]) -> Result<Vec<Item>, WriteError> {
        let nd = node_at(&*self.st, obj, level == 1)?;
        let old = items(&*self.st, &nd)?;
        self.kill(obj);
        if level == 1 {
            return self.leaf_merge(old, ops);
        }
        // Child i takes the ops below separator i+1; child 0 also takes every op below its own.
        let mut cuts = Vec::with_capacity(old.len());
        let mut rest = ops;
        for i in 0..old.len() {
            let cut = match old.get(i + 1) {
                Some((nk, _)) => rest.partition_point(|o| key_of(o) < nk.as_slice()),
                None => rest.len(),
            };
            let (mine, tail) = rest.split_at(cut);
            cuts.push(mine);
            rest = tail;
        }
        let mut slots = Vec::with_capacity(old.len());
        for ((k, child), mine) in old.into_iter().zip(cuts) {
            slots.push(if mine.is_empty() { Slot::Old(k, child) } else { Slot::New(self.apply(child, level - 1, mine)?) });
        }
        self.rebalance(slots, level - 1)
    }

    fn open_slot(&mut self, s: Slot, leaf: bool) -> Result<Vec<Item>, WriteError> {
        match s {
            Slot::New(v) => Ok(v),
            Slot::Old(_, a) => {
                let nd = node_at(&*self.st, a, leaf)?;
                self.kill(a);
                Ok(items(&*self.st, &nd)?)
            }
        }
    }

    /// Merge each small rewritten child into a neighbour, then write every rewritten child (split
    /// evenly when over the fanout). Returns the parent's new slots.
    fn rebalance(&mut self, mut slots: Vec<Slot>, child_level: usize) -> Result<Vec<Item>, WriteError> {
        let leaf = child_level == 1;
        let min = (self.b / 4).max(2);
        slots.retain(|s| !matches!(s, Slot::New(v) if v.is_empty()));
        let mut i = 0;
        while i < slots.len() {
            let small = matches!(slots.get(i), Some(Slot::New(v)) if v.len() < min);
            if !small || slots.len() < 2 {
                i += 1;
                continue;
            }
            let a = i.saturating_sub(1);
            let right = slots.remove(a + 1);
            let left = slots.remove(a);
            let mut merged = self.open_slot(left, leaf)?;
            merged.extend(self.open_slot(right, leaf)?);
            slots.insert(a, Slot::New(merged));
            i = a;
        }
        let mut out = Vec::with_capacity(slots.len());
        for s in slots {
            match s {
                Slot::Old(k, a) => out.push((k, a)),
                Slot::New(v) => {
                    for piece in split_even(v, self.b) {
                        let first = piece.first().map(|(k, _)| k.clone()).unwrap_or_default();
                        let a = self.write_node(&piece, leaf)?;
                        out.push((first, a));
                    }
                }
            }
        }
        Ok(out)
    }

    /// The root from the top level's slots: grow while over the fanout, collapse one-child chains.
    /// Returns (top node, DEPTH); (0, 0) is the empty map.
    pub fn finish(&mut self, mut its: Vec<Item>, mut level: usize) -> Result<(usize, usize), WriteError> {
        loop {
            if its.is_empty() {
                return Ok((0, 0));
            }
            if level > MAX_TREE_DEPTH {
                return Err(WriteError::Tree(TreeError::Root { why: "DEPTH past MAX_TREE_DEPTH" }));
            }
            if its.len() > self.b {
                let mut up = Vec::new();
                for piece in split_even(its, self.b) {
                    let first = piece.first().map(|(k, _)| k.clone()).unwrap_or_default();
                    up.push((first, self.write_node(&piece, level == 1)?));
                }
                its = up;
                level += 1;
                continue;
            }
            if level > 1 && its.len() == 1 {
                let mut top = its.first().map_or(0, |(_, a)| *a);
                level -= 1;
                while level > 1 {
                    let nd = node_at(&*self.st, top, false)?;
                    if nd.m != 1 {
                        break;
                    }
                    self.kill(top);
                    top = ref_at(&*self.st, &nd, 0)?;
                    level -= 1;
                }
                return Ok((top, level));
            }
            return Ok((self.write_node(&its, level == 1)?, level));
        }
    }

    /// The v4 root, written last.
    pub fn write_root(&mut self, n: i64, top: usize, depth: usize) -> Result<usize, WriteError> {
        let r = self.st.alloc(&mut self.tx, ROOT_V4 as i64, DIGEST_KV_ROOT)?;
        for (i, v) in [n, 0, 0, 0, 0, VERSION_TREE].into_iter().enumerate() {
            self.st.put_cell(r, i, v);
        }
        if top == 0 {
            self.st.put_cell(r, 6, 0);
        } else {
            self.st.link(r, 6, top);
        }
        self.st.put_cell(r, 7, depth as i64);
        self.st.seal(r);
        Ok(r)
    }
}

/// Refuse a fanout outside the format's range.
pub(crate) fn check_fanout(b: usize) -> Result<(), WriteError> {
    if (MIN_FANOUT..=MAX_FANOUT).contains(&b) { Ok(()) } else { Err(WriteError::Fanout(b)) }
}
