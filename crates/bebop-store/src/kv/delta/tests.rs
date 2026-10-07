//! Tests for the delta chain (W-DELTA). One named corrupted cell per defect (no fuzzing,
//! operator rule); the seeded sequences are a fixed, named set of 300 deterministic edit
//! scripts asserting one invariant (replay == compaction), not a crash hunt.
use super::*;
use crate::kv::{DIGEST_ARR_I64, VERSION};
use crate::Store;
use crate::kv::golden_tests::golden_catalogue;
use crate::kv::zc::{kv_prefix_in, KvIn};
use crate::verify::KvError;
use crate::View;

fn base(kv: &Kv) -> Store {
    Store::from_bytes(&kv.compacted_bytes_fit(16 << 20).unwrap())
}

/// Append `ops` to `st`, mirror them into `model`; compact both ways when the writer says so.
fn apply(st: &mut Store, model: &mut Kv, ops: &[Op]) -> Appended {
    for op in ops {
        match op {
            Op::Put(k, v) => model.put(k, v),
            Op::Remove(k) => {
                model.remove(k);
            }
        }
    }
    let a = append_delta(st, ops).unwrap();
    if let Appended::Compact(_) = a {
        *st = Store::from_bytes(&model.compacted_bytes_fit(16 << 20).unwrap());
    }
    a
}

/// Everything a reader can say about an image, through every reader, against `model`.
fn same_as(st: &Store, model: &Kv, what: &str) {
    let b = st.to_bytes_trimmed();
    let back = Kv::load(&Store::from_bytes(&b)).unwrap_or_else(|| panic!("{what}: Kv::load refused"));
    assert_eq!(back.entries, model.entries, "{what}: Kv::load");
    let v = View::new(&b);
    let zc = KvIn::open(&v).unwrap_or_else(|e| panic!("{what}: KvIn::open {e:?}"));
    assert_eq!(zc.len(), model.entries.len(), "{what}: len");
    for (i, (k, val)) in model.entries.iter().enumerate() {
        assert_eq!(&*zc.key(i).unwrap(), k.as_bytes(), "{what}: key {i}");
        assert_eq!(&*zc.value(i).unwrap(), &val[..], "{what}: value {i}");
        assert_eq!(zc.get(k.as_bytes()).unwrap().as_deref(), Some(&val[..]), "{what}: get {k}");
    }
    assert_eq!(zc.snapshot_root_u64().unwrap(), model.snapshot_root_u64(), "{what}: fold");
    let again = KvIn::reopen(&v, zc.checked()).unwrap();
    assert_eq!(again.snapshot_root_u64().unwrap(), model.snapshot_root_u64(), "{what}: reopen fold");
    let want: Vec<(Vec<u8>, Vec<u8>)> =
        model.entries.iter().filter(|(k, _)| k.starts_with("p:")).map(|(k, v)| (k.as_bytes().to_vec(), v.clone())).collect();
    assert_eq!(kv_prefix_in(&v, b"p:").unwrap(), want, "{what}: prefix");
}

fn small(n: usize) -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    for i in 0..n {
        kv.put(&format!("p:{i:02}"), format!("value-{i}-{}", "x".repeat(100 + i % 11)).as_bytes());
    }
    kv.put("loc", b"here");
    kv
}

#[test]
fn a_price_edit_appends_one_record_and_reads_back_everywhere() {
    let kv = golden_catalogue();
    let mut st = base(&kv);
    let mut model = Kv { entries: kv.entries.clone() };
    let mut v = model.get("product:d007").unwrap();
    v.extend_from_slice(b" price 950");
    let a = apply(&mut st, &mut model, &[Op::Put("product:d007", &v)]);
    assert!(matches!(a, Appended::Delta(_)), "{a:?}");
    assert_eq!(Kv::version(&st), VERSION_DELTA);
    same_as(&st, &model, "one price edit");
    // and the compacted image of what was read is the same catalogue, byte-equal fold
    let back = Kv::load(&st).unwrap();
    let compact = Kv::load(&Store::from_bytes(&back.compacted_bytes_fit(16 << 20).unwrap())).unwrap();
    assert_eq!(compact.snapshot_root_u64(), model.snapshot_root_u64(), "fold after compaction");
    assert_eq!(compact.entries, model.entries);
}

/// RED target "replay skips the newest delta": the same key edited twice answers the SECOND.
#[test]
fn the_newest_of_two_edits_to_one_key_wins() {
    let mut st = base(&small(10));
    let mut model = small(10);
    assert!(matches!(apply(&mut st, &mut model, &[Op::Put("p:03", b"first")]), Appended::Delta(_)));
    assert!(matches!(apply(&mut st, &mut model, &[Op::Put("p:03", b"second")]), Appended::Delta(_)));
    assert_eq!(Kv::load(&st).unwrap().get("p:03").as_deref(), Some(&b"second"[..]), "Kv::load");
    let b = st.to_bytes_trimmed();
    assert_eq!(KvIn::open(View::new(&b)).unwrap().get(b"p:03").unwrap().as_deref(), Some(&b"second"[..]), "KvIn");
    same_as(&st, &model, "two edits");
}

/// RED target "compaction drops a remove": a key removed by a delta stays removed through a
/// read and a compaction of what was read.
#[test]
fn a_removed_key_stays_removed_through_compaction() {
    let mut st = base(&small(10));
    let mut model = small(10);
    apply(&mut st, &mut model, &[Op::Remove("p:04"), Op::Put("p:zz", b"new")]);
    same_as(&st, &model, "remove + put");
    let read = Kv::load(&st).unwrap();
    assert!(read.get("p:04").is_none(), "the removed key came back on read");
    let compact = Kv::load(&Store::from_bytes(&read.compacted_bytes_fit(1 << 20).unwrap())).unwrap();
    assert!(compact.get("p:04").is_none(), "compaction brought the removed key back");
    assert_eq!(compact.entries, model.entries);
}

/// RED target "crc not checked on a delta": one changed byte of a record's value is a
/// `BadCrc` naming THAT record -- from `Kv::load_checked` and from `KvIn::open`.
#[test]
fn a_changed_byte_in_a_delta_record_is_a_bad_crc_naming_it() {
    let mut st = base(&small(10));
    let mut model = small(10);
    apply(&mut st, &mut model, &[Op::Put("p:05", b"eight-bytes-and-more")]);
    let rec = chain_in(&st, st.root().unwrap()).unwrap()[0];
    let mut bad = Store::from_bytes(&st.to_bytes_trimmed());
    bad.cells[rec.obj + 2 + rec.val_cell()] ^= 1 << 9;
    match Kv::load_checked(&bad) {
        Err(KvError::BadCrc(b)) => assert_eq!(b.obj, rec.obj, "named the wrong object"),
        other => panic!("a changed delta byte was read: {:?}", other.map(|k| k.entries.len())),
    }
    let b = bad.to_bytes_trimmed();
    assert!(matches!(KvIn::open(View::new(&b)), Err(KvError::BadCrc(x)) if x.obj == rec.obj), "KvIn::open read it");
    // the positive twin: the untouched image reads
    assert!(Kv::load_checked(&st).is_ok());
}

/// The chain's claims, one named corrupted cell each: every one is refused, none panics.
#[test]
fn a_chain_that_does_not_hold_is_refused_cell_by_cell() {
    let mut st = base(&small(16));
    let mut model = small(16);
    assert!(matches!(apply(&mut st, &mut model, &[Op::Put("p:01", b"a")]), Appended::Delta(_)));
    assert!(matches!(apply(&mut st, &mut model, &[Op::Put("p:02", b"b")]), Appended::Delta(_)));
    let root = st.root().unwrap();
    let rec = chain_in(&st, root).unwrap()[0];
    let cases: [(&str, usize, i64); 7] = [
        ("D claims one record more", root + 2 + 7, 3),
        ("D claims one record fewer", root + 2 + 7, 1),
        ("D is negative", root + 2 + 7, -1),
        ("an unknown op", rec.obj + 2 + 1, 7),
        ("KLEN disagrees with the record's length", rec.obj + 2 + 2, 40),
        ("a remove that carries a value", rec.obj + 2 + 1, OP_REMOVE),
        ("NEXT nulled, so D is a lie", rec.obj + 2, 0),
    ];
    for (name, cell, val) in cases {
        let mut bad = Store::from_bytes(&st.to_bytes_trimmed());
        bad.cells[cell] = val;
        // re-seal so the crc agrees: the STRUCTURE must refuse, not the crc
        let in_rec = cell >= rec.obj && cell < rec.obj + 2 + st.obj_cells(rec.obj);
        bad.seal(if in_rec { rec.obj } else { root });
        assert!(chain_in(&bad, root).is_none(), "{name}: the chain was believed");
        assert!(Kv::load(&bad).is_none(), "{name}: Kv::load read it");
        let b = bad.to_bytes_trimmed();
        assert!(KvIn::open(View::new(&b)).is_err(), "{name}: KvIn read it");
    }
    assert!(chain_in(&st, root).is_some(), "the positive twin");
}

/// A self-loop that keeps the count honest is ended by the cell budget, not by a hang.
#[test]
fn a_looping_chain_ends_and_is_refused() {
    let mut st = base(&small(16));
    let mut model = small(16);
    assert!(matches!(apply(&mut st, &mut model, &[Op::Put("p:01", b"a")]), Appended::Delta(_)));
    assert!(matches!(apply(&mut st, &mut model, &[Op::Put("p:02", b"b")]), Appended::Delta(_)));
    let root = st.root().unwrap();
    let chain = chain_in(&st, root).unwrap();
    let (newest, oldest) = (chain[0].obj, chain[1].obj);
    let mut bad = Store::from_bytes(&st.to_bytes_trimmed());
    bad.cells[root + 2 + 7] = 1 << 40; // a count that never runs out
    bad.seal(root);
    bad.link(oldest, 0, newest); // oldest -> newest -> oldest -> ...
    bad.seal(oldest);
    assert!(chain_in(&bad, root).is_none(), "a looping chain was believed");
    assert!(Kv::load(&bad).is_none());
}

/// A v3 root needs its eight cells; a six-cell root that says 3 is not one.
#[test]
fn a_six_cell_root_saying_version_3_is_refused() {
    let mut st = base(&small(3));
    let root = st.root().unwrap();
    st.cells[root + 2 + 5] = VERSION_DELTA;
    st.seal(root);
    assert!(Kv::load(&st).is_none());
    let b = st.to_bytes_trimmed();
    assert!(KvIn::open(View::new(&b)).is_err());
}

/// The bytes a delta changes: the two superblock pages (cells < ARENA) and the arena from the
/// old cursor on. NOTHING in between -- which is what makes a DO write two chunks, not all.
#[test]
fn a_delta_changes_only_the_superblock_page_and_the_tail() {
    let kv = golden_catalogue();
    let mut st = base(&kv);
    let before = st.to_bytes_trimmed();
    let cursor = st.pick().unwrap().arena_used as usize;
    let mut model = Kv { entries: kv.entries.clone() };
    apply(&mut st, &mut model, &[Op::Put("product:d100", b"{\"price\":1}")]);
    let after = st.to_bytes_trimmed();
    assert!(after.len() > before.len());
    let first = (0..before.len()).find(|&i| before[i] != after[i]).unwrap();
    let last = (crate::ARENA * 8..before.len()).rev().find(|&i| before[i] != after[i]);
    assert!(first < crate::ARENA * 8, "the first changed byte {first} is past the superblock pages");
    assert_eq!(last, None, "a byte of the old arena changed");
    assert!(cursor * 8 <= before.len());
}

#[test]
fn superseded_cells_are_written_and_compaction_clears_them() {
    let mut st = base(&small(10));
    assert_eq!(st.pick().unwrap().superseded_cells, 0, "a compacted image has none");
    let root_cells = 2 + st.obj_cells(st.root().unwrap()) as i64;
    let mut model = small(10);
    apply(&mut st, &mut model, &[Op::Put("p:03", b"x")]);
    let sb = st.pick().unwrap();
    // the old root (exact) + p:03's old value cells (estimate: ceil(len/8))
    let old = small(10).get("p:03").unwrap().len().div_ceil(8) as i64;
    assert_eq!(sb.superseded_cells, root_cells + old);
    let rec = chain_in(&st, st.root().unwrap()).unwrap()[0];
    let rec_cells = 2 + st.obj_cells(rec.obj) as i64;
    let sup1 = sb.superseded_cells;
    let root2 = 2 + st.obj_cells(st.root().unwrap()) as i64;
    apply(&mut st, &mut model, &[Op::Put("p:03", b"y")]);
    assert_eq!(st.pick().unwrap().superseded_cells, sup1 + root2 + rec_cells, "a shadowed record is exact");
}

#[test]
fn the_writer_asks_for_compaction_when_it_should() {
    let mut st = base(&small(10));
    let ops: Vec<Op> = (0..MAX_DELTAS + 1).map(|_| Op::Put("p:01", b"v")).collect();
    assert_eq!(append_delta(&mut st, &ops).unwrap(), Appended::Compact(Why::Deltas));
    assert!(matches!(append_delta_with(&mut st, &ops[..MAX_DELTAS], MAX_DELTAS, 0).unwrap(), Appended::Delta(_)), "exactly MAX is allowed");
    let mut st = base(&small(10));
    assert_eq!(append_delta_with(&mut st, &[Op::Put("p:01", b"v")], MAX_DELTAS, 1 << 40).unwrap(), Appended::Compact(Why::Dead));
    // v1: the eager v1 writer's image
    let mut v1 = Store::create_bytes(1 << 20).unwrap();
    let (tx, root) = Kv::stage_init_v(&mut v1, 1).unwrap();
    v1.commit_bytes(&tx, root);
    assert_eq!(append_delta(&mut v1, &[Op::Put("k", b"v")]).unwrap(), Appended::Compact(Why::V1));
    // full: a fresh image trimmed to its own content has one doubling of headroom at most
    let mut tight = Store::create_bytes(crate::MIN_BYTES + 64 * 8).unwrap();
    let (tx, root) = small(0).stage_write(&mut tight, VERSION, DIGEST_ARR_I64, super::super::DIGEST_KV_ROOT).unwrap();
    tight.commit_bytes(&tx, root);
    let big = vec![7u8; 4096];
    assert_eq!(append_delta(&mut tight, &[Op::Put("k", &big)]).unwrap(), Appended::Compact(Why::Full));
    assert!(Kv::load(&tight).is_some(), "a refused append left the image readable");
}

/// THE EQUIVALENCE, on 300 fixed seeds (the 1000-sequence pass mark runs through `Catalog` in
/// dowiz-hub): after every batch the image reads -- through Kv::load, KvIn and a compaction of
/// what was read -- exactly as the model, which is what compacting would have written.
#[test]
fn replay_equals_the_model_on_300_seeded_edit_sequences() {
    let mut compactions = 0;
    for seed in 0..300u64 {
        let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut r = |n: u64| {
            x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (x >> 33) % n
        };
        let mut model = small(12);
        let mut st = base(&model);
        for batch in 0..(1 + r(12)) {
            let mut owned: Vec<(String, Option<Vec<u8>>)> = Vec::new();
            for _ in 0..(1 + r(3)) {
                let k = format!("p:{:02}", r(16));
                if r(4) == 0 {
                    owned.push((k, None));
                } else {
                    owned.push((k, Some(format!("s{seed}b{batch}-{}", "y".repeat(r(20) as usize)).into_bytes())));
                }
            }
            let ops: Vec<Op> = owned.iter().map(|(k, v)| match v { Some(v) => Op::Put(k, v), None => Op::Remove(k) }).collect();
            if let Appended::Compact(_) = apply(&mut st, &mut model, &ops) {
                compactions += 1;
            }
            same_as(&st, &model, &format!("seed {seed} batch {batch}"));
        }
        let read = Kv::load(&st).unwrap();
        let compact = Kv::load(&Store::from_bytes(&read.compacted_bytes_fit(1 << 20).unwrap())).unwrap();
        assert_eq!(compact.snapshot_root_u64(), model.snapshot_root_u64(), "seed {seed}: fold after compaction");
    }
    println!("replay_equals_the_model: 300 sequences, {compactions} compactions on the way");
    assert!(compactions > 0, "no sequence reached a compaction: the trigger was never exercised");
}
