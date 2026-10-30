//! W-ES-P8C: the enumerated crash harness over v4 TREE commits (`Writer::commit_tree`). Every
//! power-loss state of every scenario recovers generation k or k+1, replaying (`Kv::load_checked`,
//! which reads v4 since P8c) to exactly the map before or the map with the ops applied by hand --
//! a `BTreeMap`, never the tree code.

use std::collections::BTreeMap;

use super::crash::{Model, Tear};
use super::harness::{run, Opts, Report};
use super::testkit::{next, replay, Map};
use super::{SimDisk, SimOp};
use crate::kv::delta::KeyOp;
use crate::kv::tree::{tree_compacted, TreeIn, FANOUT};
use crate::{Durability, Store, Writer};

const POWER: Opts = Opts { model: Model::Power(Tear::Half), pick_only: false, old_intact: true };

fn key(i: usize) -> String {
    format!("k{i:03}")
}

/// A v4 image of keys 0..n (values 20 bytes), and its map.
fn base(n: usize) -> (Store, Map) {
    let m: BTreeMap<String, Vec<u8>> = (0..n).map(|i| (key(i), vec![b'a' + (i % 26) as u8; 20])).collect();
    let b = tree_compacted(m.iter().map(|(k, v)| (k.as_bytes(), v.as_slice())), 1 << 22, FANOUT, None).unwrap();
    (Store::from_bytes(&b), Some(m))
}

type Owned = Vec<(Vec<u8>, Option<Vec<u8>>)>;

/// The ops (sorted, one per key, as `Batch::ops` gives them, owned) and `before` with them applied.
fn ops_of(before: &Map, puts: &[usize], dels: &[usize]) -> (Owned, Map) {
    let mut ops: BTreeMap<String, Option<Vec<u8>>> = BTreeMap::new();
    for &i in puts {
        ops.insert(key(i), Some(format!("new-{i}").into_bytes()));
    }
    for &i in dels {
        ops.insert(key(i), None);
    }
    let mut m = before.clone().unwrap_or_default();
    for (k, v) in &ops {
        match v {
            Some(v) => m.insert(k.clone(), v.clone()),
            None => m.remove(k),
        };
    }
    (ops.into_iter().map(|(k, v)| (k.into_bytes(), v)).collect(), Some(m))
}

/// The borrowed `KeyOp`s of owned ops.
fn kops(o: &Owned) -> Vec<KeyOp<'_>> {
    o.iter().map(|(k, v)| match v {
        Some(v) => KeyOp::Put(k, v),
        None => KeyOp::Remove(k),
    }).collect()
}

/// One tree commit flushed with `Data` over a `SimDisk`: (durable cells, the writer's ops, k).
fn flushed(st: &Store, ops: &[KeyOp]) -> (Vec<i64>, Vec<SimOp>, i64) {
    let mut w = Writer::open(SimDisk::new(st.cells.clone())).unwrap();
    let k = w.durable().gen;
    assert_eq!(w.commit_tree(ops, FANOUT, Durability::Data).unwrap(), k + 1);
    (st.cells.clone(), w.into_device().ops().to_vec(), k)
}

fn check(r: &Report, what: &str) {
    assert!(r.green(), "{what}: {} of {} states fail; first {:?}", r.failures.len(), r.states, r.first());
    assert!(r.states >= 8, "{what}: {} states", r.states);
}

/// The scenarios: a few puts and a remove across leaves; inserts that split leaves and grow a
/// level; removes that merge leaves and shrink the tree; the first commit onto an empty v4 map.
#[test]
fn every_crash_point_of_a_tree_commit_recovers_k_or_k_plus_1() {
    let (b200, m200) = base(200);
    let (b40, m40) = base(40);
    let (b0, m0) = base(0);
    let few = ops_of(&m200, &[3, 77, 150], &[100]);
    let split = ops_of(&m40, &(1000..1100).collect::<Vec<_>>(), &[]);
    let shrink = ops_of(&m200, &[], &(0..190).collect::<Vec<_>>());
    let first = ops_of(&m0, &[1, 2, 3], &[]);
    for (name, st, before, (ops, after)) in [
        ("puts and a remove across leaves", &b200, &m200, few),
        ("inserts that split and grow a level", &b40, &m40, split),
        ("removes that merge and shrink", &b200, &m200, shrink),
        ("first commit onto an empty v4 map", &b0, &m0, first),
    ] {
        let (cells, sops, k) = flushed(st, &kops(&ops));
        let expect = [(k, before.clone()), (k + 1, after.clone())];
        for opts in [POWER, Opts { pick_only: true, ..POWER }, Opts { model: Model::Power(Tear::Cells), ..POWER }] {
            let r = run(&cells, &sops, opts, &expect, &replay, &next);
            check(&r, name);
            eprintln!("P8c v4 crash {name} {opts:?}: {} states green", r.states);
        }
    }
}

/// W-ES-P11: N tree commits staged (`Writer::stage_tree`) and flushed as ONE group cost two
/// barriers for every N in 1..8 (the flip), and every power-loss state recovers k or k+N --
/// never a generation between -- with the exact map.
#[test]
fn a_group_of_tree_commits_is_two_barriers_and_recovers_k_or_k_plus_n() {
    let (st, m0) = base(200);
    for n in 1..=8usize {
        let mut m = m0.clone();
        let mut staged = Vec::new();
        for i in 0..n {
            let (ops, after) = ops_of(&m, &[300 + i, 3 + i], &[10 + i]);
            staged.push(ops);
            m = after;
        }
        let mut w = Writer::open(SimDisk::new(st.cells.clone())).unwrap();
        let k = w.durable().gen;
        for ops in &staged {
            w.stage_tree(&kops(ops), FANOUT).unwrap();
        }
        assert_eq!(w.flush(Durability::Data).unwrap(), k + n as i64, "N={n}");
        let sops = w.into_device().ops().to_vec();
        assert_eq!(sops.iter().filter(|o| **o == SimOp::Barrier).count(), 2, "N={n}: barriers");
        let expect = [(k, m0.clone()), (k + n as i64, m)];
        let r = run(&st.cells, &sops, POWER, &expect, &replay, &next);
        check(&r, &format!("v4 group N={n}"));
        eprintln!("P11 v4 group N={n}: {} states green, 2 barriers", r.states);
    }
}

/// RED: a writer that SKIPPED one new leaf (its cells never reach the disk). Under `pick` alone
/// the superblock names a tree whose leaf is garbage, so a state replays to no map; with the
/// anchor (`recover`), the torn commit is caught and generation k opens -- both are asserted.
#[test]
fn a_skipped_leaf_is_found_by_the_harness() {
    let (st, before) = base(200);
    let (ops, after) = ops_of(&before, &[3, 77, 150], &[100]);
    let (cells, mut sops, k) = flushed(&st, &kops(&ops));
    let mark = st.pick().unwrap().arena_used as usize;
    let mut fin = Store { cells: cells.clone() };
    for op in &sops {
        if let SimOp::Write { off, cells } = op {
            super::apply(&mut fin.cells, *off, cells);
        }
    }
    let t = TreeIn::open(&fin).unwrap();
    let leaf = t.entries().map(|_| ()).and_then(|_| first_new_leaf(&fin, mark)).expect("a new leaf");
    let len = 2 + fin.obj_len(leaf) as usize;
    for op in sops.iter_mut() {
        if let SimOp::Write { off, cells } = op {
            for i in leaf..leaf + len {
                if let Some(c) = i.checked_sub(*off).and_then(|j| cells.get_mut(j)) {
                    *c = 0;
                }
            }
        }
    }
    let expect = [(k, before), (k + 1, after)];
    let r = run(&cells, &sops, Opts { pick_only: true, ..POWER }, &expect, &replay, &next);
    assert!(!r.green(), "the harness must see the skipped leaf");
    eprintln!("P8c RED skipped leaf, pick only: {} of {} fail; first {:?}", r.failures.len(), r.states, r.first());
    let anchored = run(&cells, &sops, POWER, &[(k, expect[0].1.clone())], &replay, &next);
    assert!(anchored.green(), "the anchor must open k: {:?}", anchored.first());
}

/// The first leaf object at or past `mark` (written by the last commit).
fn first_new_leaf(st: &Store, mark: usize) -> Result<usize, crate::kv::tree::TreeError> {
    let used = st.pick().unwrap().arena_used as usize;
    let mut i = mark;
    while i < used {
        if st.obj_digest(i) == crate::kv::tree::DIGEST_KV4_LEAF {
            return Ok(i);
        }
        i += 2 + st.obj_len(i).max(0) as usize;
    }
    Err(crate::kv::tree::TreeError::NotTree)
}
