//! Shared by the W-ES-P2 tests: the four §4.3 scenarios as REAL flushes through `Writer` over
//! a `SimDisk`, and the KV replay with its expectations computed independently of the writer
//! (a `BTreeMap` with the ops applied by hand -- never the code under test).

use std::collections::BTreeMap;

use super::SimDisk;
use crate::iodev::{Durability, Writer};
use crate::kv::delta::{append_delta_with, Appended, Op, MAX_DELTAS};
use crate::kv::Kv;
use crate::Store;

pub type Map = Option<BTreeMap<String, Vec<u8>>>;

/// Image size for the small scenarios.
pub const CAP: usize = 16 * 1024;

/// What a recovered image replays to: the KV map, or None when there is no (readable) KV.
pub fn replay(st: &Store) -> Map {
    Kv::load_checked(st).ok().map(|kv| kv.entries.into_iter().collect())
}

/// `map` with `ops` applied by hand.
pub fn applied(map: &Map, ops: &[Op]) -> Map {
    let mut m = map.clone().unwrap_or_default();
    for op in ops {
        match *op {
            Op::Put(k, v) => {
                m.insert(k.to_string(), v.to_vec());
            }
            Op::Remove(k) => {
                m.remove(k);
            }
        }
    }
    Some(m)
}

/// The next commit every harness state must accept: an empty KV schema (valid from any
/// generation that has a superblock, KV or not).
pub fn next(st: &mut Store) -> Result<(), String> {
    Kv::init_bytes(st).map(|_| ()).map_err(|e| format!("{e:?}"))
}

/// A KV holding `entries`, compacted into a fresh image of `cap` bytes.
pub fn kv_image(entries: &[(&str, &str)], cap: usize) -> Store {
    let kv = Kv { entries: entries.iter().map(|(k, v)| (k.to_string(), v.as_bytes().to_vec())).collect() };
    Store::from_bytes(&kv.compacted_bytes(cap).expect("compacts"))
}

/// Stage `ops` as one delta commit. The dead-cell trigger is OFF (`dead_div` 0): these images
/// are a few dozen cells, where one superseded root already outweighs a quarter of the live
/// ones and `append_delta` would ask for a compaction instead of a delta.
pub fn delta(st: &mut Store, ops: &[Op]) {
    assert!(matches!(append_delta_with(st, ops, MAX_DELTAS, 0), Ok(Appended::Delta(_))), "a delta, not a compaction");
}

/// A KV whose live cells outweigh a few deltas' dead ones, so the public `append_delta` (and
/// `Writer::commit_delta`) deltas instead of compacting: `a` = 1 plus a 2000-byte pad.
pub fn padded() -> Store {
    kv_image(&[("a", "1"), ("pad", "x".repeat(2000).as_str())], CAP)
}

/// One recorded flush: the durable image before it, and the writer's ops.
pub struct Flushed {
    pub base: Vec<i64>,
    pub disk: SimDisk,
    pub gen: i64,
}

/// Open a writer over `base`, let `stage` stage commits, flush once with `d`.
pub fn flush(base: &Store, d: Durability, stage: impl FnOnce(&mut Store)) -> Flushed {
    let mut w = Writer::open(SimDisk::new(base.cells.clone())).expect("opens");
    stage(w.stage().expect("stages"));
    let gen = w.flush(d).expect("flushes");
    Flushed { base: base.cells.clone(), disk: w.into_device(), gen }
}

/// The scenarios' durable images and the ops each one commits.
pub struct Scenario {
    pub name: &'static str,
    pub base: Store,
    pub before: Map,
    pub stage: fn(&mut Store),
    pub after: Map,
    pub k: i64,
}

fn s1_stage(st: &mut Store) {
    *st = kv_image(&[("a", "1"), ("b", "22")], CAP);
}
const S2_OPS: [Op<'static>; 3] = [Op::Put("d", b"4444"), Op::Put("a", b"one"), Op::Remove("b")];
fn s2_stage(st: &mut Store) {
    delta(st, &S2_OPS);
}
fn s3_entries() -> Vec<(String, Vec<u8>)> {
    (0..40).map(|i| (format!("dish-{i:02}"), vec![b'a' + (i % 26) as u8; 100])).collect()
}
fn s3_stage(st: &mut Store) {
    *st = Store::from_bytes(&Kv { entries: s3_entries() }.compacted_bytes(64 * 1024).expect("compacts"));
}
fn s4_stage(st: &mut Store) {
    delta(st, &[Op::Remove("only")]);
}

/// T-ES-20's four: first commit into a fresh image; a delta on v3; a compaction into a new
/// image (40 entries, past `EXHAUSTIVE` sectors: the named classes); removing the last key.
pub fn scenarios() -> Vec<Scenario> {
    let mut v3 = kv_image(&[("a", "1"), ("b", "22")], CAP);
    delta(&mut v3, &[Op::Put("c", b"333")]);
    let v3_map = replay(&v3);
    let only = kv_image(&[("only", "x")], CAP);
    vec![
        Scenario {
            name: "first commit into a fresh image",
            base: Store::create_bytes(CAP).unwrap(),
            before: None,
            stage: s1_stage,
            after: applied(&None, &[Op::Put("a", b"1"), Op::Put("b", b"22")]),
            k: 0,
        },
        Scenario { name: "delta on v3", after: applied(&v3_map, &S2_OPS), before: v3_map, base: v3, stage: s2_stage, k: 2 },
        Scenario {
            name: "compaction into a new image",
            base: Store::create_bytes(64 * 1024).unwrap(),
            before: None,
            stage: s3_stage,
            after: Some(s3_entries().into_iter().collect()),
            k: 0,
        },
        Scenario { name: "remove the last key", before: replay(&only), base: only, stage: s4_stage, after: Some(BTreeMap::new()), k: 1 },
    ]
}
