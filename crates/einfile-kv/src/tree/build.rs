//! BULK LOAD (design §1.3): a fresh v4 tree from a sorted entry stream -- the migration of a v1-v3
//! image, and the shape a compaction (P13a) writes. Values first, then the leaves filled to the
//! fanout and split evenly, then each level above; the root last. The fold law: the tree's
//! entries, and so its fold, are the source's, byte for byte (`build` tests).

use alloc::vec::Vec;

use super::write::{check_fanout, W, WriteError};
use crate::{Kv, Store, StoreError, Tx};

/// Stage the entries (strictly ascending keys) as a fresh v4 tree in `st`, which must have no root.
pub fn stage_build<'e, I>(st: &mut Store, entries: I, fanout: usize) -> Result<(Tx, usize), WriteError>
where
    I: Iterator<Item = (&'e [u8], &'e [u8])>,
{
    check_fanout(fanout)?;
    if st.root().is_some() {
        return Err(WriteError::NotFresh);
    }
    let tx = st.begin()?;
    let mut w = W { st, tx, b: fanout, dead: 0, dn: 0 };
    let mut its: Vec<(Vec<u8>, usize)> = Vec::new();
    for (k, v) in entries {
        crate::check_entry(k.len(), v.len())?;
        if its.last().is_some_and(|(p, _)| p.as_slice() >= k) {
            return Err(WriteError::Unsorted);
        }
        let a = w.write_val(v)?;
        its.push((k.to_vec(), a));
    }
    let n = its.len() as i64;
    let (top, depth) = w.finish(its, 1)?;
    let root = w.write_root(n, top, depth)?;
    Ok((w.tx, root))
}

/// The entries as a fresh v4 image in the SMALLEST arena that holds them, doubling from 16 KiB up
/// to `max` bytes (`compacted_from_fit`'s loop), trimmed.
pub fn tree_bytes_fit<'e, I>(entries: I, max: usize, fanout: usize) -> Result<Vec<u8>, WriteError>
where
    I: Iterator<Item = (&'e [u8], &'e [u8])> + Clone,
{
    let mut cap = 16 * 1024;
    loop {
        let mut st = Store::create_bytes(cap)?;
        match stage_build(&mut st, entries.clone(), fanout) {
            Ok((tx, root)) => {
                st.commit_bytes(&tx, root);
                return Ok(st.to_bytes_trimmed());
            }
            Err(WriteError::Store(StoreError::ArenaFull { .. })) if cap < max => cap = (cap * 2).min(max),
            Err(e) => return Err(e),
        }
    }
}

/// UP-MIGRATION v1/v2/v3 -> v4: the source read through `Kv::load_checked` (every crc, the chain
/// replayed), bulk-loaded into a fresh image. A v4 source is read too (W-ES-P8C) and rebuilt.
pub fn migrate_to_tree(src: &Store, max: usize, fanout: usize) -> Result<Vec<u8>, WriteError> {
    let kv = Kv::load_checked(src).map_err(WriteError::Kv)?;
    tree_bytes_fit(kv.entries.iter().map(|(k, v)| (k.as_bytes(), v.as_slice())), max, fanout)
}
