//! TREE SEMANTICS = THE ABSTRACT MAP, on a NAMED set of op sequences (no generators): after every
//! commit the tree (get, every scan, the fold, `verify`) equals a `BTreeMap` given the same ops, and
//! every OLDER root still reads exactly its own map (persistence), its cells untouched. Each
//! sequence runs at fanout 4 (deep trees: many splits, merges, collapses), 5 (odd) and 32.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::{format, vec};
use core::ops::Bound;

use super::{commit_tree_bytes, TreeIn};
use crate::{Batch, Kv, Limits, Store, ARENA};

pub(super) type Model = BTreeMap<Vec<u8>, Vec<u8>>;
/// One commit: (key, Some(value) = put | None = remove), applied in order (the batch keeps the last).
pub(super) type Commit = Vec<(Vec<u8>, Option<Vec<u8>>)>;

pub(super) fn k(i: usize) -> Vec<u8> {
    format!("k{i:05}").into_bytes()
}
/// A value whose length varies with i and that differs per generation g.
pub(super) fn v(i: usize, g: usize) -> Vec<u8> {
    format!("{g}:{i}:{}", "x".repeat(i % 40)).into_bytes()
}
pub(super) fn put(i: usize, g: usize) -> (Vec<u8>, Option<Vec<u8>>) {
    (k(i), Some(v(i, g)))
}
pub(super) fn del(i: usize) -> (Vec<u8>, Option<Vec<u8>>) {
    (k(i), None)
}

/// Commit `c` through the real `Batch` and the real writer.
pub(super) fn commit(st: &mut Store, c: &Commit, b: usize) {
    let mut batch = Batch::new(Limits::FORMAT);
    for (key, val) in c {
        match val {
            Some(x) => batch.put(key, x).unwrap(),
            None => batch.remove(key).unwrap(),
        }
    }
    let ops: Vec<_> = batch.ops().collect();
    commit_tree_bytes(st, &ops, b).unwrap();
}

/// The fold `Kv` gives the same entries (every key here is UTF-8).
fn model_fold(m: &Model) -> u64 {
    let entries = m.iter().map(|(a, b)| (String::from_utf8(a.clone()).unwrap(), b.clone())).collect();
    Kv { entries }.snapshot_root_u64()
}

/// The tree at `root` IS `m`: verify, len, every entry, get of every key and of absent ones, the
/// fold, and scans over bounds drawn from the model's own keys.
pub(super) fn check(st: &Store, root: usize, m: &Model) {
    let t = TreeIn::at(st, root).unwrap();
    t.verify().unwrap();
    assert_eq!(t.len(), m.len());
    let want: Vec<(Vec<u8>, Vec<u8>)> = m.iter().map(|(a, b)| (a.clone(), b.clone())).collect();
    assert_eq!(t.entries().unwrap(), want);
    for (key, val) in m {
        assert_eq!(t.get(key).unwrap().as_deref(), Some(val.as_slice()));
        let mut miss = key.clone();
        miss.push(0);
        assert_eq!(t.get(&miss).unwrap().is_some(), m.contains_key(&miss));
    }
    for absent in [&b""[..], b"a", b"k", b"z", b"k99999z"] {
        assert_eq!(t.get(absent).unwrap().as_deref(), m.get(absent).map(|x| x.as_slice()));
    }
    assert_eq!(t.fold().unwrap(), model_fold(m));
    let keys: Vec<&Vec<u8>> = m.keys().collect();
    let picks: Vec<&[u8]> = [0, keys.len() / 3, keys.len() / 2, keys.len().saturating_sub(1)]
        .iter()
        .filter_map(|&i| keys.get(i).map(|x| x.as_slice()))
        .chain([&b"k0"[..], b"k00100x"])
        .collect();
    for &lo in &picks {
        for &hi in &picks {
            if lo > hi {
                continue;
            }
            for (bl, bh) in [(Bound::Included(lo), Bound::Included(hi)), (Bound::Excluded(lo), Bound::Unbounded), (Bound::Included(lo), Bound::Excluded(hi))] {
                if bl == Bound::Excluded(lo) && bh == Bound::Excluded(lo) {
                    continue;
                }
                let got: Vec<(Vec<u8>, Vec<u8>)> = t.range(bl, bh).map(|e| e.map(|(a, b)| (a.into_owned(), b.into_owned())).unwrap()).collect();
                let want: Vec<(Vec<u8>, Vec<u8>)> = m.range::<[u8], _>((bl, bh)).map(|(a, b)| (a.clone(), b.clone())).collect();
                assert_eq!(got, want, "range {bl:?}..{bh:?}");
            }
        }
    }
    for p in [&b"k00"[..], b"k001", b"k0001", b"", b"q"] {
        let got = t.prefix(p).map(|e| e.unwrap().0.into_owned()).collect::<Vec<_>>();
        let want = m.keys().filter(|x| x.starts_with(p)).cloned().collect::<Vec<_>>();
        assert_eq!(got, want, "prefix {p:?}");
    }
}

/// Run a named sequence at fanout `b`: check after every commit, then every older root again.
pub(super) fn run(name: &str, seq: &[Commit], b: usize) {
    let mut st = Store::create_bytes(64 << 20).unwrap();
    let mut m = Model::new();
    let mut hist: Vec<(usize, Model)> = Vec::new();
    for c in seq {
        let used = st.pick().map_or(ARENA as i64, |s| s.arena_used) as usize;
        let before: Vec<i64> = st.cells.get(ARENA..used).unwrap().to_vec();
        commit(&mut st, c, b);
        assert_eq!(st.cells.get(ARENA..used).unwrap(), &before[..], "{name} b={b}: a commit wrote below its mark");
        for (key, val) in c {
            match val {
                Some(x) => m.insert(key.clone(), x.clone()),
                None => m.remove(key),
            };
        }
        let root = st.root().unwrap();
        check(&st, root, &m);
        hist.push((root, m.clone()));
    }
    for (root, old) in &hist {
        check(&st, *root, old);
    }
}

fn all(name: &str, seq: Vec<Commit>) {
    for b in [4, 5, 32] {
        run(name, &seq, b);
    }
}

#[test]
fn asc_insert_one_per_commit() {
    all("asc", (0..300).map(|i| vec![put(i, 0)]).collect());
}

#[test]
fn desc_insert_one_per_commit() {
    all("desc", (0..300).rev().map(|i| vec![put(i, 0)]).collect());
}

#[test]
fn stride_insert_then_overwrite_all() {
    let mut s: Vec<Commit> = (0..301).map(|i| vec![put(i * 7 % 301, 0)]).collect();
    s.push((0..301).map(|i| put(i, 1)).collect());
    all("stride", s);
}

#[test]
fn one_big_commit_then_remove_all_in_one() {
    all("big", vec![(0..1000).map(|i| put(i, 0)).collect(), (0..1000).map(del).collect(), vec![put(5, 2)]]);
}

#[test]
fn build_then_remove_one_per_commit_ascending() {
    let mut s: Vec<Commit> = vec![(0..200).map(|i| put(i, 0)).collect()];
    s.extend((0..200).map(|i| vec![del(i)]));
    all("rm-asc", s);
}

#[test]
fn build_then_remove_one_per_commit_from_the_middle_out() {
    let mut s: Vec<Commit> = vec![(0..150).map(|i| put(i, 0)).collect()];
    s.extend((0..75).map(|d| vec![del(75 + d), del(74 - d)]));
    all("rm-mid", s);
}

#[test]
fn remove_every_other_then_reinsert() {
    let mut s: Vec<Commit> = vec![(0..400).map(|i| put(i, 0)).collect()];
    s.extend((0..400).step_by(2).collect::<Vec<_>>().chunks(13).map(|c| c.iter().map(|&i| del(i)).collect()));
    s.extend((0..400).step_by(2).collect::<Vec<_>>().chunks(29).map(|c| c.iter().map(|&i| put(i, 3)).collect()));
    all("evens", s);
}

#[test]
fn removes_of_absent_keys_change_nothing() {
    all("absent", vec![(0..50).map(|i| put(i * 2, 0)).collect(), (0..50).map(|i| del(i * 2 + 1)).collect(), vec![del(1000)], vec![]]);
}

#[test]
fn mixed_batches_put_remove_and_same_key_twice() {
    let s: Vec<Commit> = (0..40)
        .map(|g| {
            let mut c: Commit = (0..17).map(|j| put((g * 31 + j * 11) % 250, g)).collect();
            c.extend((0..9).map(|j| del((g * 13 + j * 7) % 250)));
            c.push(put(g * 3 % 250, g + 100));
            c.push(del(g * 3 % 250));
            c
        })
        .collect();
    all("mixed", s);
}

#[test]
fn shrink_to_one_entry_then_regrow() {
    let mut s: Vec<Commit> = vec![(0..500).map(|i| put(i, 0)).collect()];
    s.push((1..500).map(del).collect());
    s.extend((0..60).map(|i| vec![put(1000 + i, 1)]));
    s.push((0..2000).map(|i| put(i, 9)).collect());
    all("shrink", s);
}

/// The conformance fixture `kv4del` (crates/bebop-wasm/fixtures/conform/src/kv4.rs): 150 keys put by
/// 6 interleaved commits, then 3 commits that remove 110 of them. W-ES-P8E: at fanout 4 the writer
/// left a REACHABLE one-child node (a node whose other children emptied and whose survivor shrank
/// under the minimum had nothing to merge with, so it was written as it was), which `verify`
/// refuses with `Node { why: "a reachable node with one child" }`. P8D sidestepped it with B = 8.
#[test]
fn kv4del_at_every_fanout() {
    let key = |i: usize| format!("k/{:03}{}", (i * 37) % 150, if i % 7 == 0 { "/longer-tail" } else { "" }).into_bytes();
    let val = |i: usize, round: usize| -> Vec<u8> {
        let n = [0usize, 1, 7, 8, 9, 40][(i + round) % 6];
        (0..n).map(|j| (i * 13 + j * 5 + round) as u8).collect()
    };
    let mut s: Vec<Commit> = (0..6).map(|r| (0..150).filter(|i| i % 6 == r).map(|i| (key(i), Some(val(i, r)))).collect()).collect();
    for r in 0..3 {
        let mut c: Commit = (0..150).filter(|i| i % 4 != 0 && i % 3 == r).map(|i| (key(i), None)).collect();
        c.extend((0..150).filter(|i| i % 4 == 0 && i % 30 == r * 4).map(|i| (key(i), Some(val(i, 9)))));
        s.push(c);
    }
    for b in [4, 5, 6, 8, 32] {
        run("kv4del", &s, b);
    }
}

/// The smallest shape of the same defect: a parent whose children all but one are emptied while
/// the survivor shrinks below the minimum, in ONE commit, at the minimum fanout.
#[test]
fn a_node_whose_siblings_empty_and_whose_survivor_shrinks_is_merged_with_a_cousin() {
    // Fanout 4: 256 keys in one commit = 64 leaves, 16 nodes, 4 nodes, 1 root (depth 4). A lone
    // one-entry LEAF is legal; the defect needs the survivor to be an INTERNAL node, so the tree
    // must be at least 4 deep and the emptied subtree one of the root's children.
    let mut s: Vec<Commit> = vec![(0..256).map(|i| put(i, 0)).collect()];
    // Everything under the root's second child but one entry, and everything under its third.
    s.push((65..192).map(del).collect());
    s.push((0..256).map(del).collect());
    s.push((0..256).map(|i| put(i, 1)).collect());
    all("cousin", s);
}

#[test]
fn edge_keys_and_value_lengths() {
    let long = "L".repeat(1000).into_bytes();
    let mut c: Commit = vec![
        (b"".to_vec(), Some(b"empty key".to_vec())),
        (b"a".to_vec(), Some(vec![])),
        (b"ab".to_vec(), Some(vec![7; 8])),
        (b"abc".to_vec(), Some(vec![9; 9])),
        ("ÿÿ".as_bytes().to_vec(), Some(vec![1; 300])),
        (long.clone(), Some(vec![2; 7])),
    ];
    c.extend((0..40).map(|i| put(i, 0)));
    all("edge", vec![c, vec![(b"".to_vec(), None), (long, Some(vec![]))], vec![(b"a".to_vec(), None), (b"".to_vec(), Some(vec![0; 65]))]]);
}
