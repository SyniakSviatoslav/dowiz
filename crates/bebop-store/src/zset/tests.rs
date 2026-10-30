//! T-LV-INC-01: a KV image read as a Z-set -- the base plus its delta chain -- integrates to
//! exactly the entries `Kv::load` replays, byte for byte; and the group laws the step relies on.

use super::*;
use crate::kv::delta::{append_delta_with, Appended, Op, MAX_DELTAS};
use crate::kv::Kv;
use crate::{root_in, Store};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../bebop-wasm/fixtures/{name}.store", env!("CARGO_MANIFEST_DIR"))).expect("fixture")
}

/// `Kv::load`'s entries, as a Z-set of weight +1 rows.
fn loaded(st: &Store) -> ZSet<KvRow> {
    let kv = Kv::load(st).expect("a KV image");
    ZSet::from_pairs(kv.entries.into_iter().map(|(k, v)| ((k.into_bytes(), v), 1)).collect())
}

/// A v2 base of `entries`, compacted into a fresh image (the writer's own path).
pub(crate) fn base(entries: &[(&str, &str)]) -> Store {
    let mut kv = Kv { entries: Vec::new() };
    for (k, v) in entries {
        kv.put(k, v.as_bytes());
    }
    Store::from_bytes(&kv.compacted_bytes(1 << 20).unwrap())
}

/// Append `ops` as delta records, the dead-cell trigger off (`dead_div` 0, as kv3.store was
/// made): these tests pin the chain, not the compaction policy.
pub(crate) fn delta(st: &mut Store, ops: &[Op]) {
    assert!(matches!(append_delta_with(st, ops, MAX_DELTAS, 0).unwrap(), Appended::Delta(_)), "{ops:?}");
}

fn check(st: &Store, what: &str) {
    let root = root_in(st).unwrap();
    assert_eq!(from_kv(st, root).expect(what), loaded(st), "{what}: base ⊕ Δ == Kv::load");
    // And step by step: the newest k records' Z-set is the difference of two prefixes.
    let d = chain_in(st, root).unwrap().len();
    let b = from_kv_base(st, root).unwrap();
    assert_eq!(b.add(&from_kv_chain(st, root, d).unwrap()), loaded(st), "{what}: one Δ");
}

#[test]
fn kv_chain_integrates_to_kv_load() {
    for f in ["kv", "kv2", "kv3"] {
        check(&Store::from_bytes(&fixture(f)), f);
    }
    let script: &[&[Op]] = &[
        &[Op::Put("b", b"x")],                       // overwrite: -(b,22) +(b,x)
        &[Op::Put("d", b"4")],                       // a new key
        &[Op::Remove("a")],                          // a removal: -(a,1)
        &[Op::Remove("zz")],                         // removing an absent key changes nothing
        &[Op::Put("c", b"333")],                     // the same value: +1 -1 = 0
        &[Op::Put("b", b"y"), Op::Put("b", b"z")],   // two records on one key in one commit
        &[Op::Put("a", b"back"), Op::Remove("d")],   // a removed key returns
    ];
    let mut st = base(&[("a", "1"), ("b", "22"), ("c", "333")]);
    check(&st, "the base alone");
    for (i, ops) in script.iter().enumerate() {
        delta(&mut st, ops);
        check(&st, &format!("after step {i}"));
    }
    // The longest chain the writer builds (MAX_DELTAS = 32), each record a price edit.
    let mut st = base(&[("dish/001", "100"), ("dish/002", "200")]);
    for i in 0..30 {
        delta(&mut st, &[Op::Put(if i % 2 == 0 { "dish/001" } else { "dish/002" }, format!("{}", 100 + i).as_bytes())]);
    }
    check(&st, "30 price edits");
    let root = root_in(&st).unwrap();
    let last = from_kv_chain(&st, root, 1).unwrap();
    assert_eq!(last.len(), 2, "one edit = one row out, one row in");
    assert_eq!(last.rows().iter().map(|r| r.1).sum::<i64>(), 0, "an edit keeps the count");
}

/// The group laws: a + (-a) = 0, zero weights are never kept, add commutes -- and the
/// positive twin: a non-zero sum survives.
#[test]
fn add_neg_and_consolidation() {
    let a = ZSet::from_pairs(vec![(1, 2), (3, -1), (1, -2), (5, 4)]);
    assert_eq!(a.rows(), &[(3, -1), (5, 4)], "consolidated, the zero row dropped");
    assert!(a.add(&a.neg()).is_zero(), "a + (-a) = 0");
    let b = ZSet::from_pairs(vec![(3, 1), (7, 1)]);
    assert_eq!(a.add(&b), b.add(&a));
    assert_eq!(a.add(&b).rows(), &[(5, 4), (7, 1)], "3 cancelled, the rest kept");
    assert_eq!(a.sub(&a), ZSet::new());
    assert_eq!(a.weight(&5), 4);
    assert_eq!(a.weight(&9), 0);
}

#[test]
fn log_tail_is_plus_one() {
    let r = |b: u8| Record { id: [b; 32], prev: [0; 32], actor_pubkey: [0; 32], actor_seq: b as u64, flags: 0, payload: vec![b] };
    let z = from_log_tail(&[r(1), r(2), r(3)]);
    assert_eq!(z.rows().iter().map(|p| p.1).collect::<Vec<_>>(), vec![1, 1, 1]);
    assert_eq!(z.weight(&[2; 32]), 1);
}

/// The digest IS the module header's text: FNV-1a 64 over each row's cells and weight as
/// 8 little-endian bytes, rows ascending. Recomputed here byte by byte, not through `digest`.
#[test]
fn digest_is_the_spec_text() {
    let z = ZSet::from_pairs(vec![(vec![5, -1], 1), (vec![-3, 9], 2)]);
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for v in [-3i64, 9, 2, 5, -1, 1] {
        for b in v.to_le_bytes() {
            h = (h ^ b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    assert_eq!(digest(&z), h as i64);
    assert_ne!(digest(&z), digest(&ZSet::from_pairs(vec![(vec![5, -1], 1), (vec![-3, 9], 1)])), "a weight moves it");
}

/// A KV row's cells are the node keys of 'O' ‖ len8 ‖ bytes (nodekey.rs, K-3's framing).
#[test]
fn kv_row_cells_are_object_node_keys() {
    let mut f = vec![b'O'];
    f.extend_from_slice(&3u64.to_le_bytes());
    f.extend_from_slice(b"abc");
    let c = kv_row_cells(&(b"abc".to_vec(), Vec::new()));
    assert_eq!(c[0], crate::nodekey::k64(&f));
    assert_eq!(c[1], crate::nodekey::k64(&[b'O', 0, 0, 0, 0, 0, 0, 0, 0]));
}
