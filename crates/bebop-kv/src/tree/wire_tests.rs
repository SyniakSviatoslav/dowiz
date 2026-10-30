//! P8c: the library's readers over v4 (`KvIn`, `Kv::load_checked`, `Staged`/`MergeCursor`), the
//! copy compaction and its trigger. Every expectation is a `BTreeMap` or a decoded `Kv` built
//! independently of the tree code.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::ops::Bound;

use super::compact::{dead_over, tree_compacted};
use super::seq_tests::{commit, del, k, put, v};
use super::{TreeError, TreeIn, FANOUT};
use crate::verify::KvError;
use crate::zc::KvIn;
use crate::{Batch, Kv, Limits, Staged, Store, View};

/// 300 entries (depth 2 at B=32) as a fresh v4 image's bytes, and the same entries as a map.
fn image() -> (Vec<u8>, BTreeMap<Vec<u8>, Vec<u8>>) {
    let m: BTreeMap<Vec<u8>, Vec<u8>> = (0..300).map(|i| (k(i), v(i, 0))).collect();
    let b = tree_compacted(m.iter().map(|(a, b)| (a.as_slice(), b.as_slice())), 1 << 24, FANOUT, None).unwrap();
    (b, m)
}

fn kv_of(m: &BTreeMap<Vec<u8>, Vec<u8>>) -> Kv {
    Kv { entries: m.iter().map(|(a, b)| (alloc::string::String::from_utf8(a.clone()).unwrap(), b.clone())).collect() }
}

#[test]
fn kvin_and_load_checked_read_v4() {
    let (b, m) = image();
    let st = Store::from_bytes(&b);
    assert_eq!(TreeIn::open(&st).unwrap().depth(), 2);
    let kv = Kv::load_checked(&st).unwrap();
    assert_eq!(kv.entries, kv_of(&m).entries);
    let want_fold = kv_of(&m).snapshot_root_u64();
    for zc in [KvIn::open(View::new(&b)).unwrap(), KvIn::open_lazy(View::new(&b)).unwrap()] {
        assert_eq!(zc.len(), 300);
        assert_eq!(zc.snapshot_root_u64().unwrap(), want_fold);
        for (key, val) in &m {
            assert_eq!(zc.get(key).unwrap().as_deref(), Some(&val[..]));
        }
        assert_eq!(zc.get(b"k99999").unwrap(), None);
        assert!(zc.get(&k(7)).unwrap().is_some_and(|c| matches!(c, alloc::borrow::Cow::Borrowed(_))), "zero-copy");
        assert_eq!(zc.key(0).map(|_| ()), Err(KvError::Tree(TreeError::Unindexed)));
        assert_eq!(zc.prefix_range(b"k").map(|_| ()), Err(KvError::Tree(TreeError::Unindexed)));
    }
    let ck = KvIn::open(View::new(&b)).unwrap().checked();
    assert_eq!(KvIn::reopen(View::new(&b), ck).unwrap().snapshot_root_u64().unwrap(), want_fold);
}

/// The merged read (a batch over a v4 base) answers what the map with the batch applied holds,
/// for every range shape -- `MergeCursor`'s v4 path.
#[test]
fn staged_cursor_over_a_v4_base_merges_the_batch() {
    let (b, mut m) = image();
    let zc = KvIn::open(View::new(&b)).unwrap();
    let mut batch = Batch::new(Limits::FORMAT);
    for i in [0usize, 5, 150, 299, 400] {
        batch.put(&k(i), b"new").unwrap();
        m.insert(k(i), b"new".to_vec());
    }
    for i in [1usize, 151, 298, 777] {
        batch.remove(&k(i)).unwrap();
        m.remove(&k(i));
    }
    let s = Staged::new(&zc, &batch);
    assert_eq!(s.len().unwrap(), m.len());
    assert_eq!(s.fold_root().unwrap(), kv_of(&m).snapshot_root_u64());
    let (lo, hi) = (k(140), k(160));
    let shapes: [(Bound<&[u8]>, Bound<&[u8]>); 4] = [
        (Bound::Included(&lo), Bound::Excluded(&hi)),
        (Bound::Excluded(&lo), Bound::Included(&hi)),
        (Bound::Unbounded, Bound::Included(&lo)),
        (Bound::Included(&hi), Bound::Unbounded),
    ];
    for (a, z) in shapes {
        let got: Vec<(Vec<u8>, Vec<u8>)> = s.cursor(a, z).unwrap().map(|e| e.map(|(x, y)| (x.into_owned(), y.into_owned())).unwrap()).collect();
        let want: Vec<(Vec<u8>, Vec<u8>)> = m.range::<[u8], _>((a, z)).map(|(x, y)| (x.clone(), y.clone())).collect();
        assert_eq!(got, want, "{a:?}..{z:?}");
    }
}

/// A corrupted v4 image is refused by `open` and `load_checked` with the tree's own reason.
#[test]
fn a_torn_v4_leaf_is_refused_by_name() {
    let (mut b, _) = image();
    let st = Store::from_bytes(&b);
    let t = TreeIn::open(&st).unwrap();
    let (top, _) = t.head();
    let leaf = super::node::ref_at(&st, &super::node::node_at(&st, top, false).unwrap(), 0).unwrap();
    let at = (leaf + 3) * 8;
    b[at] ^= 1;
    let bad = Store::from_bytes(&b);
    assert!(matches!(Kv::load_checked(&bad), Err(KvError::Tree(TreeError::BadCrc(_)))), "{:?}", Kv::load_checked(&bad).map(|_| ()));
    assert!(matches!(KvIn::open(View::new(&b)).map(|_| ()), Err(KvError::Tree(TreeError::BadCrc(_)))));
}

/// The compaction keeps the generation monotone, leaves headroom, and the trigger is the dead
/// ratio: a fresh image has none dead; 40 one-key commits kill more than they keep at 300 keys.
#[test]
fn compaction_is_dead_ratio_triggered_and_generation_monotone() {
    let (b, m) = image();
    let mut st = Store::from_bytes(&b);
    assert!(!dead_over(&st, 1), "a fresh image has no dead cells");
    let mut n = 0;
    while !dead_over(&st, 1) {
        commit(&mut st, &vec![put(n % 300, 1)], FANOUT);
        n += 1;
        assert!(n < 10_000, "the trigger never fired");
    }
    assert!(n > 1, "one commit must not already trigger at 300 keys: n={n}");
    commit(&mut st, &vec![del(3)], FANOUT);
    let g = st.pick().unwrap().generation;
    let live = TreeIn::open(&st).unwrap().entries().unwrap();
    let c = tree_compacted(live.iter().map(|(a, b)| (a.as_slice(), b.as_slice())), 1 << 24, FANOUT, Some(g)).unwrap();
    let cs = Store::from_bytes(&c);
    let sb = cs.pick().unwrap();
    assert_eq!(sb.generation, g + 1);
    assert_eq!(sb.superseded_cells, 0);
    assert!(sb.arena_used as usize * 2 <= cs.cells.len(), "headroom: used {} of {} cells", sb.arena_used, cs.cells.len());
    assert_eq!(Kv::load_checked(&cs).unwrap().entries.len(), m.len() - 1);
    eprintln!("{}", format!("P8c trigger: {n} one-key commits at 300 keys"));
}
