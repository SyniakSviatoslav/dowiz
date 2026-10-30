//! One tree commit: refuse what must be refused BEFORE `begin`, path-copy the ops, write the root.

use alloc::vec::Vec;

use super::read::TreeIn;
use super::write::{check_fanout, key_of, WriteError, W};
use super::TreeError;
use crate::delta::KeyOp;
use crate::{Store, Tx};

/// Stage one commit of `ops` (strictly ascending, one per key) onto the image's v4 tree -- or onto
/// the empty map when the store has no root yet. Returns the transaction and the new KV root; the
/// caller commits (`commit_tree_bytes`, or P8c's `Writer::commit_tree`).
pub fn stage_tree(st: &mut Store, ops: &[KeyOp], fanout: usize) -> Result<(Tx, usize), WriteError> {
    check_fanout(fanout)?;
    if !ops.iter().zip(ops.iter().skip(1)).all(|(a, b)| key_of(a) < key_of(b)) {
        return Err(WriteError::Unsorted);
    }
    for op in ops {
        match op {
            KeyOp::Put(k, v) => crate::check_entry(k.len(), v.len())?,
            KeyOp::Remove(k) => crate::check_entry(k.len(), 0)?,
        }
    }
    let (n0, top, depth, old_root) = match st.root() {
        None => (0, 0, 0, None),
        Some(r) => match TreeIn::at(&*st, r) {
            Ok(t) => (t.len() as i64, t.top(), t.depth(), Some(r)),
            Err(TreeError::NotTree) => return Err(WriteError::NotTree { v: crate::Kv::version(st) }),
            Err(e) => return Err(WriteError::Tree(e)),
        },
    };
    let tx = st.begin()?;
    let mut w = W { st, tx, b: fanout, dead: 0, dn: 0 };
    if let Some(r) = old_root {
        w.kill(r);
    }
    let (its, level) = if depth == 0 { (w.leaf_merge(Vec::new(), ops)?, 1) } else { (w.apply(top, depth, ops)?, depth) };
    let (top, depth) = w.finish(its, level)?;
    let n = n0 + w.dn;
    if n < 0 {
        return Err(WriteError::Tree(TreeError::Count { claim: n0, got: n }));
    }
    let root = w.write_root(n, top, depth)?;
    let mut tx = w.tx;
    tx.sup_delta += w.dead;
    Ok((tx, root))
}

/// `stage_tree` and commit in memory; returns the new generation.
pub fn commit_tree_bytes(st: &mut Store, ops: &[KeyOp], fanout: usize) -> Result<i64, WriteError> {
    let (tx, root) = stage_tree(st, ops, fanout)?;
    Ok(st.commit_bytes(&tx, root))
}
