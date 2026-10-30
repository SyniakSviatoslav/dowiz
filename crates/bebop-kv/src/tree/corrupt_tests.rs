//! REFUSALS: one named corruption per rule (no fuzzers in this repo), each next to its positive
//! twin on the same image before the cell was touched.

use super::node::{key_at, node_at, ref_at};
use super::seq_tests::{commit, k, put};
use super::*;
use crate::Store;

/// 60 entries at fanout 4: depth 3.
fn tree() -> (Store, usize) {
    let mut st = Store::create_bytes(4 << 20).unwrap();
    commit(&mut st, &(0..60).map(|i| put(i, 0)).collect(), MIN_FANOUT);
    let r = st.root().unwrap();
    assert_eq!(TreeIn::at(&st, r).unwrap().depth(), 3);
    (st, r)
}

fn patch(st: &mut Store, obj: usize, cell: usize, v: i64) {
    st.put_cell(obj, cell, v);
    st.seal(obj);
}

/// The leftmost leaf under the tree at `root`.
fn first_leaf(st: &Store, root: usize) -> usize {
    let t = TreeIn::at(st, root).unwrap();
    let mut obj = t.top();
    for _ in 1..t.depth() {
        obj = ref_at(st, &node_at(st, obj, false).unwrap(), 0).unwrap();
    }
    obj
}

#[test]
fn a_forward_ref_is_refused() {
    let (mut st, r1) = tree();
    commit(&mut st, &vec![put(1000, 1)], MIN_FANOUT);
    let t1 = TreeIn::at(&st, r1).unwrap();
    assert!(t1.get(&k(0)).unwrap().is_some());
    t1.verify().unwrap();
    // gen 2's rewritten last child of the top: a well-formed node one level down, LATER in the arena.
    let t2 = TreeIn::open(&st).unwrap();
    let top2 = node_at(&st, t2.top(), false).unwrap();
    let later = ref_at(&st, &top2, top2.m - 1).unwrap();
    let top1 = t1.top();
    assert!(later > top1);
    let cell = node_at(&st, top1, false).unwrap().r_cell(0);
    patch(&mut st, top1, cell, (later - top1) as i64);
    let t1 = TreeIn::at(&st, r1).unwrap();
    assert_eq!(t1.get(&k(0)).err(), Some(TreeError::Forward { obj: top1 }));
    assert_eq!(t1.verify().err(), Some(TreeError::Forward { obj: top1 }));
}

#[test]
fn a_depth_claim_that_does_not_hold_is_refused() {
    for d in [2i64, 4] {
        let (mut st, r) = tree();
        assert!(TreeIn::at(&st, r).unwrap().get(&k(5)).unwrap().is_some());
        patch(&mut st, r, 7, d);
        let t = TreeIn::at(&st, r).unwrap();
        assert!(matches!(t.get(&k(5)), Err(TreeError::Depth { .. })), "DEPTH {d}");
        assert!(matches!(t.verify(), Err(TreeError::Depth { .. })), "DEPTH {d}");
    }
}

#[test]
fn a_leaf_key_past_the_next_separator_is_refused() {
    let (mut st, r) = tree();
    TreeIn::at(&st, r).unwrap().verify().unwrap();
    let leaf = first_leaf(&st, r);
    let nd = node_at(&st, leaf, true).unwrap();
    // "k0000x" -> "k9000x": still the largest key IN its leaf, now past every later leaf.
    let (a, _) = key_at(&st, &nd, nd.m - 1).unwrap();
    let b = a + 1;
    let cell = st.get(leaf, b / 8 - leaf - 2) as u64;
    let sh = 8 * (b % 8);
    let new = (cell & !(0xFF << sh)) | ((b'9' as u64) << sh);
    patch(&mut st, leaf, b / 8 - leaf - 2, new as i64);
    // Refused at the NEXT leaf: its first key is the first one below the patched key.
    let next = TreeIn::at(&st, r).unwrap().verify().err();
    assert!(matches!(next, Some(TreeError::Order { obj }) if obj != leaf), "{next:?}");
}

#[test]
fn an_n_claim_that_does_not_hold_is_refused() {
    let (mut st, r) = tree();
    TreeIn::at(&st, r).unwrap().verify().unwrap();
    patch(&mut st, r, 0, 61);
    assert_eq!(TreeIn::at(&st, r).unwrap().verify().err(), Some(TreeError::Count { claim: 61, got: 60 }));
}

#[test]
fn a_torn_value_fails_its_own_crc() {
    let (mut st, r) = tree();
    let leaf = first_leaf(&st, r);
    let val = ref_at(&st, &node_at(&st, leaf, true).unwrap(), 0).unwrap();
    TreeIn::at(&st, r).unwrap().verify().unwrap();
    st.put_cell(val, 1, st.get(val, 1) ^ 1);
    assert!(matches!(TreeIn::at(&st, r).unwrap().verify(), Err(TreeError::BadCrc(b)) if b.obj == val));
}

#[test]
fn a_torn_node_fails_its_crc() {
    let (mut st, r) = tree();
    let leaf = first_leaf(&st, r);
    let nd = node_at(&st, leaf, true).unwrap();
    TreeIn::at(&st, r).unwrap().verify().unwrap();
    st.put_cell(leaf, nd.karea, st.get(leaf, nd.karea) ^ (1 << 40));
    assert!(matches!(TreeIn::at(&st, r).unwrap().verify(), Err(TreeError::BadCrc(b)) if b.obj == leaf));
}

#[test]
fn a_root_that_breaks_its_shape_is_refused() {
    let (mut st, r) = tree();
    patch(&mut st, r, 2, 5);
    assert_eq!(TreeIn::at(&st, r).err(), Some(TreeError::Root { why: "cells 1-4 of a v4 root must be zero" }));
    let (mut st, r) = tree();
    patch(&mut st, r, 7, 0);
    assert_eq!(TreeIn::at(&st, r).err(), Some(TreeError::Root { why: "DEPTH 0 is the empty map: no TREE, n = 0" }));
    let (mut st, r) = tree();
    patch(&mut st, r, 6, 3);
    assert_eq!(TreeIn::at(&st, r).err(), Some(TreeError::Forward { obj: r }));
}
