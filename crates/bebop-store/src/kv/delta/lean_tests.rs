//! The Lean model's theorems on the REAL code (W-DELTALEAN, 2026-10-09).
//!
//! bebop-lang/formal/Bebop/KvDelta.lean proves, for EVERY chain of an ABSTRACT model (a map, put/remove
//! batches, replay = left fold, compact = the folded image re-emitted as one batch):
//!   replay_compact       replay (compact c) = replay c
//!   compact_idem         compact (compact c) = compact c
//!   compact_prefix       replay (compact c1 ++ c2) = replay (c1 ++ c2)   (compact_append_comm is c2 = [d])
//! The model knows nothing of bytes. This test is the bridge: it runs the real writer
//! (`append_delta_with`) and the real compactor (`Kv::compacted_bytes_fit` of what `Kv::load_checked`
//! read) over a FIXED set of NAMED chains (no fuzzing: operator rule) and asserts the same equalities,
//! plus that the real read equals `lean_replay` -- the model's `replay`, transcribed. What the model does
//! not cover (byte layout, crc, superseded cells, the compaction trigger, refusals) is listed in the
//! .lean file's header; the crc and the refusals have their own one-cell tests in `tests.rs`.
use super::*;
use crate::kv::golden_tests::golden_catalogue;
use crate::Store;
use std::collections::BTreeMap;

/// One op: `(key, Some(value))` puts, `(key, None)` removes. A batch is one `append_delta` call.
type Batch = Vec<(String, Option<Vec<u8>>)>;

fn p(k: &str, v: &str) -> (String, Option<Vec<u8>>) {
    (k.to_string(), Some(v.as_bytes().to_vec()))
}
fn r(k: &str) -> (String, Option<Vec<u8>>) {
    (k.to_string(), None)
}

/// KvDelta.lean `replay`, transcribed: the batches oldest first, the ops in order, onto `base`.
fn lean_replay(base: &Kv, chain: &[Batch]) -> Vec<(String, Vec<u8>)> {
    let mut m: BTreeMap<String, Vec<u8>> = base.entries.iter().cloned().collect();
    for (k, v) in chain.iter().flatten() {
        match v {
            Some(v) => {
                m.insert(k.clone(), v.clone());
            }
            None => {
                m.remove(k);
            }
        }
    }
    m.into_iter().collect()
}

/// An image with room to append: the base compacted into a 1 MiB arena.
fn image_of(kv: &Kv) -> Store {
    Store::from_bytes(&kv.compacted_bytes(1 << 20).unwrap())
}

/// Append every batch as delta records. The trigger is switched off (`usize::MAX`, `DEAD_DIV` 0) so the
/// chain really is a chain: a `Compact` answer here is a failure, not a path.
fn append_all(st: &mut Store, chain: &[Batch], what: &str) {
    for (i, b) in chain.iter().enumerate() {
        let ops: Vec<Op> = b
            .iter()
            .map(|(k, v)| match v {
                Some(v) => Op::Put(k, v),
                None => Op::Remove(k),
            })
            .collect();
        let a = append_delta_with(st, &ops, usize::MAX, 0).unwrap();
        assert!(
            matches!(a, Appended::Delta(_)),
            "{what}: batch {i} was not appended: {a:?}"
        );
    }
}

/// READ, through the checked reader (crc of the root and every record).
fn read(st: &Store, what: &str) -> Kv {
    Kv::load_checked(&Store::from_bytes(&st.to_bytes_trimmed()))
        .unwrap_or_else(|e| panic!("{what}: load_checked {e:?}"))
}

/// COMPACT what was read: the bytes the writer would write instead of a delta.
fn compact(kv: &Kv) -> Vec<u8> {
    kv.compacted_bytes_fit(16 << 20).unwrap()
}

fn check(name: &str, base: &Kv, chain: &[Batch]) {
    let mut st = image_of(base);
    append_all(&mut st, chain, name);
    if !chain.is_empty() {
        assert_eq!(Kv::version(&st), VERSION_DELTA, "{name}: not a v3 image");
        assert_eq!(
            chain_in(&st, st.root().unwrap()).unwrap().len(),
            chain.iter().map(Vec::len).sum::<usize>(),
            "{name}: one record per op"
        );
    }
    let got = read(&st, name);
    // the real read is the model's replay
    assert_eq!(
        got.entries,
        lean_replay(base, chain),
        "{name}: read != lean_replay"
    );
    // replay_compact: the compacted image reads as the chain did, and it is a v2 image with no chain
    let cb = compact(&got);
    let cst = Store::from_bytes(&cb);
    assert_eq!(
        Kv::version(&cst),
        crate::kv::VERSION,
        "{name}: compaction did not write v2"
    );
    let back = read(&cst, name);
    assert_eq!(back.entries, got.entries, "{name}: replay_compact");
    assert_eq!(
        back.snapshot_root_u64(),
        got.snapshot_root_u64(),
        "{name}: fold after compaction"
    );
    // compact_idem, on the BYTES (stronger than the model's list equality)
    assert_eq!(compact(&back), cb, "{name}: compact_idem");
    // compact_prefix at EVERY split (compact_append_comm is the last one): compact c[..i], append c[i..]
    for i in 0..=chain.len() {
        let mut pre = image_of(base);
        append_all(&mut pre, &chain[..i], name);
        let mut st2 = image_of(&read(&pre, name));
        append_all(&mut st2, &chain[i..], name);
        let got2 = read(&st2, name);
        assert_eq!(
            got2.entries, got.entries,
            "{name}: compact_prefix at split {i}"
        );
        assert_eq!(
            compact(&got2),
            cb,
            "{name}: compact_prefix at split {i}, compacted bytes"
        );
    }
}

fn named_chains() -> Vec<(&'static str, Vec<Batch>)> {
    let long: Vec<Batch> = (0..40)
        .map(|i| {
            if i % 7 == 6 {
                vec![r(&format!("k{}", i % 5))]
            } else {
                vec![p(&format!("k{}", i % 5), &format!("v{i}"))]
            }
        })
        .collect();
    vec![
        ("the empty chain", vec![]),
        ("one put", vec![vec![p("a", "1")]]),
        (
            "overwrite across batches (newest_put_wins)",
            vec![vec![p("a", "first")], vec![p("a", "second")]],
        ),
        (
            "overwrite inside one batch",
            vec![vec![p("a", "first"), p("a", "second")]],
        ),
        (
            "put then remove (remove_sticks)",
            vec![vec![p("a", "x")], vec![r("a")]],
        ),
        (
            "put and remove in one batch",
            vec![vec![p("a", "x"), r("a")]],
        ),
        (
            "remove of an absent key (remove_absent_noop)",
            vec![vec![p("a", "x")], vec![r("zz")]],
        ),
        (
            "remove then put again",
            vec![vec![p("a", "x")], vec![r("a")], vec![p("a", "back")]],
        ),
        (
            "an empty batch in the middle",
            vec![vec![p("a", "x")], vec![], vec![p("b", "y")]],
        ),
        (
            "every key removed",
            vec![vec![p("a", "1"), p("b", "2")], vec![r("a"), r("b")]],
        ),
        (
            "empty, one-cell and multi-cell values",
            vec![
                vec![p("e", ""), p("s", "8 bytes!"), p("m", &"m".repeat(70))],
                vec![p("e", &"z".repeat(9)), p("m", "")],
            ],
        ),
        (
            "keys inserted out of order, shared prefixes",
            vec![
                vec![p("b", "1")],
                vec![p("a", "2")],
                vec![p("ab", "3")],
                vec![p("a\u{e9}", "4")],
                vec![r("ab"), p("aa", "5")],
            ],
        ),
        ("40 batches, past MAX_DELTAS", long),
    ]
}

#[test]
fn the_lean_theorems_hold_on_the_real_writer_over_named_chains() {
    let empty = Kv {
        entries: Vec::new(),
    };
    let chains = named_chains();
    for (name, chain) in &chains {
        check(name, &empty, chain);
    }
    assert!(
        chains.iter().any(|(_, c)| c.len() > MAX_DELTAS),
        "no chain is longer than the writer's trigger"
    );
    println!(
        "lean_tests: {} named chains on the empty base",
        chains.len()
    );
}

/// The same theorems on the real catalogue's base: a price edit, a removed dish, a new dish.
#[test]
fn the_lean_theorems_hold_on_the_golden_catalogue_base() {
    let kv = golden_catalogue();
    let mut edited = kv.get("product:d007").unwrap();
    edited.extend_from_slice(b" price 950");
    let edited = String::from_utf8(edited).unwrap();
    let chain: Vec<Batch> = vec![
        vec![p("product:d007", &edited)],
        vec![r("product:d003"), p("product:d100", "{\"price\":1}")],
        vec![p("product:d007", "{\"price\":2}")],
    ];
    check("golden catalogue edits", &kv, &chain);
}
