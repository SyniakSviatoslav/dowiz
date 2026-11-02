//! W-ES-HARD: THE CRASH STATES OF A PUBLISH (a compaction into a sibling file). Synced: the old
//! file or the new, both recover. Deferred (`Durability::None`): until the next barrier the name
//! holds exactly the last durable generation; a Data commit made before a None compaction
//! SURVIVES every power-loss state, and the RED twin -- the mutant device that renames an
//! unsynced sibling at once -- is caught by name: the new file empty, the Data commit gone.

use super::crash::{each_state, Model, Tear};
use super::harness::{run, Opts, Report};
use super::testkit::{applied, delta, kv_image, next, replay, Map, CAP};
use super::{PublishHow, SimDisk, SimOp};
use crate::kv::delta::Op;
use crate::{Device, Durability, Store, Writer};

const POWER: Opts = Opts { model: Model::Power(Tear::Half), pick_only: false, old_intact: false };

/// Over the base (a, b, then a delta putting c), `before` commits what must be DURABLE first (the
/// disk is settled after it: the record starts there); then a compaction of the entries is
/// published with `d`; then `after` may stage and flush more. Returns the durable image, the
/// record since it, and the expected (generation, map) list: the durable one, the compaction,
/// and what `after` built.
fn published(d: Durability, before: impl FnOnce(&mut Writer<SimDisk>), after: impl FnOnce(&mut Writer<SimDisk>, &Map) -> Vec<(i64, Map)>) -> (Vec<i64>, Vec<SimOp>, Vec<(i64, Map)>) {
    let mut base = kv_image(&[("a", "1"), ("b", "22")], CAP);
    delta(&mut base, &[Op::Put("c", b"333")]);
    let mut w = Writer::open(SimDisk::new(base.cells)).unwrap();
    before(&mut w);
    let durable = w.into_device().cache();
    let mut w = Writer::open(SimDisk::new(durable.clone())).unwrap();
    let k = w.durable().gen;
    let m = replay(w.store());
    let mut expect = vec![(k, m.clone())];
    let entries = m.clone().unwrap();
    let bytes = crate::kv::compacted_from_fit(entries.iter().map(|(k, v)| (k.as_bytes(), v.as_slice())), CAP, Some(k)).unwrap();
    let staged = w.device().fresh().unwrap();
    assert_eq!(w.publish(staged, Store::from_bytes(&bytes), d).unwrap(), k + 1);
    expect.push((k + 1, m.clone()));
    expect.extend(after(&mut w, &m));
    (durable, w.into_device().ops().to_vec(), expect)
}

fn nothing(_: &mut Writer<SimDisk>, _: &Map) -> Vec<(i64, Map)> {
    vec![]
}
fn none(_: &mut Writer<SimDisk>) {}

/// One delta commit flushed with `d`; its (generation, map).
fn commit(w: &mut Writer<SimDisk>, m: &Map, key: &'static str, d: Durability) -> Vec<(i64, Map)> {
    delta(w.stage().unwrap(), &[Op::Put(key, b"x")]);
    let g = w.flush(d).unwrap();
    vec![(g, applied(m, &[Op::Put(key, b"x")]))]
}

fn run_map(base: &[i64], ops: &[SimOp], opts: Opts, expect: &[(i64, Map)]) -> Report {
    run(base, ops, opts, expect, &replay, &next)
}

fn kinds(base: &[i64], ops: &[SimOp], model: Model) -> Vec<&'static str> {
    let mut v = vec![];
    each_state(base, ops, model, |p, _| v.push(p.kind));
    v
}

/// `Data`: the sibling was barriered, the record is one synced publish, and its two states --
/// the old file, the new file -- both recover (k, then k+1).
#[test]
fn a_synced_publish_is_the_old_file_or_the_new_and_both_recover() {
    let (base, ops, expect) = published(Durability::Data, none, nothing);
    assert!(matches!(ops.as_slice(), [SimOp::Publish { how: PublishHow::Synced, .. }]), "{ops:?}");
    let r = run_map(&base, &ops, POWER, &expect);
    assert!(r.green(), "{:?}", r.first());
    assert_eq!(kinds(&base, &ops, POWER.model), ["publish: the old file", "publish: the new file"]);
    let process = run_map(&base, &ops, Opts { model: Model::Process, ..POWER }, &expect);
    assert!(process.green() && process.states == 2, "{:?}", process.first());
    eprintln!("W-HARD publish Data: {} power states + {} process states green", r.states, process.states);
}

/// `None`: the publish is deferred. Until a barrier the name holds the old file and nothing
/// else: one power-loss state, one process-crash state after the base, both the last durable
/// generation; no state is an empty file.
#[test]
fn a_deferred_publish_leaves_only_the_old_file_until_a_barrier() {
    let (base, ops, expect) = published(Durability::None, none, nothing);
    assert!(matches!(ops.as_slice(), [SimOp::Publish { how: PublishHow::Deferred, .. }]), "{ops:?}");
    let only_k = &expect[..1];
    let r = run_map(&base, &ops, POWER, only_k);
    assert!(r.green(), "{:?}", r.first());
    assert_eq!(kinds(&base, &ops, POWER.model), ["publish: the old file"]);
    let process = run_map(&base, &ops, Opts { model: Model::Process, ..POWER }, only_k);
    assert!(process.green(), "{:?}", process.first());
    assert_eq!(kinds(&base, &ops, Model::Process), ["process crash", "process crash after the deferred publish"]);
    eprintln!("W-HARD publish None deferred: {} power + {} process states, every one the old file at generation {}", r.states, process.states, only_k[0].0);
}

/// THE REQUIREMENT: a `Data` commit (k1, durable: the record starts after its barriers), then a
/// `None` compaction (k1+1) and a `None` delta after it (k1+2), then power loss at every point:
/// every state opens k1 or later -- the `expect` list holds no generation below k1, so a state
/// opening less (or refusing) fails -- and the Data commit's key is in every recovered map. THE
/// RED TWIN, by name: the same record with the publish rewritten to the mutant that renames an
/// unsynced sibling at once fails on the state "the new file EMPTY", where the Data commit is
/// gone with the whole file.
#[test]
fn a_data_commit_survives_a_none_compaction_and_power_loss() {
    let (base, ops, expect) = published(
        Durability::None,
        |w| {
            delta(w.stage().unwrap(), &[Op::Put("d", b"x")]);
            w.flush(Durability::Data).unwrap();
        },
        |w, m| commit(w, m, "e", Durability::None),
    );
    let k1 = expect[0].0;
    assert_eq!(expect.iter().map(|e| e.0).collect::<Vec<_>>(), [k1, k1 + 1, k1 + 2]);
    assert!(expect[0].1.as_ref().unwrap().contains_key("d"), "the durable base holds the Data commit");
    let from_k1 = &expect[..];
    let r = run_map(&base, &ops, POWER, from_k1);
    assert!(r.green(), "a state opened below the Data commit: {:?}", r.first());
    let mut lowest = i64::MAX;
    let states = each_state(&base, &ops, POWER.model, |p, img| {
        let rec = super::recover(&Store { cells: img.to_vec() }).unwrap_or_else(|e| panic!("{p}: {e:?}"));
        lowest = lowest.min(rec.sb.generation);
        let mut st = Store { cells: img.to_vec() };
        if let Some(h) = rec.hidden {
            st.cells[h] = 0;
        }
        assert!(replay(&st).unwrap().contains_key("d"), "{p}: the Data commit's key is missing");
    });
    assert_eq!(lowest, k1, "the lowest generation any state opens is the Data commit's");
    assert!(!kinds(&base, &ops, POWER.model).iter().any(|k| k.contains("EMPTY")), "no state is an empty file");
    eprintln!("W-HARD Data commit k+1={k1}, None compaction, None delta: {states} power-loss states, lowest generation opened {lowest}, key d in every map");
    let mutant: Vec<SimOp> = ops
        .iter()
        .map(|o| match o {
            SimOp::Publish { cells, how: PublishHow::Deferred } => SimOp::Publish { cells: cells.clone(), how: PublishHow::RenamedUnsynced },
            o => o.clone(),
        })
        .collect();
    let red = run_map(&base, &mutant, POWER, from_k1);
    assert!(!red.green(), "the mutant device must lose the Data commit");
    let first = red.first().unwrap();
    assert!(first.contains("publish renamed unsynced: the new file EMPTY") && first.contains("recover refused"), "{first}");
    eprintln!("W-HARD RED rename-at-unsynced-publish: {} of {} states fail; first {first}", red.failures.len(), red.states);
    let process = run_map(&base, &ops, Opts { model: Model::Process, ..POWER }, from_k1);
    assert!(process.green(), "{:?}", process.first());
}

/// A `Data` flush after a deferred publish names the sibling: the record is publish, the delta's
/// writes, barrier, superblock, barrier; the states at that first barrier are the old file whole
/// and the new file whole WITH the delta's writes (they went to the sibling), and every state is
/// a listed generation.
#[test]
fn the_next_barrier_names_the_deferred_sibling_with_everything_written_since() {
    let (base, ops, expect) = published(Durability::None, none, |w, m| commit(w, m, "d", Durability::Data));
    assert_eq!(ops.iter().filter(|o| **o == SimOp::Barrier).count(), 2);
    assert!(matches!(ops.first(), Some(SimOp::Publish { how: PublishHow::Deferred, .. })));
    let r = run_map(&base, &ops, POWER, &expect);
    assert!(r.green(), "{:?}", r.first());
    let k = kinds(&base, &ops, POWER.model);
    assert_eq!(&k[..3], ["publish: the old file", "publish deferred: the old file", "publish deferred: the new file"], "{k:?}");
    assert!(k.iter().all(|k| !k.contains("EMPTY")));
    let process = run_map(&base, &ops, Opts { model: Model::Process, ..POWER }, &expect);
    assert!(process.green(), "{:?}", process.first());
    eprintln!("W-HARD publish None + Data flush: {} power states green, kinds {k:?}", r.states);
}

/// A `Db` compaction into a fresh `SimDisk` is recorded, not forgotten: the record holds the
/// publish and the next commit's writes after it, and the cache is the new file plus them.
#[test]
fn a_db_compaction_over_the_sim_disk_is_recorded() {
    let base = kv_image(&[("a", "1")], CAP);
    let db = crate::Db::open(SimDisk::new(base.cells.clone()), crate::Options::default().compaction(crate::kv::delta::MAX_DELTAS, 0)).unwrap();
    let g = db.compact_fresh().unwrap().get();
    let mut w = db.begin_write().unwrap();
    w.put(b"b", b"2").unwrap();
    assert_eq!(w.commit(Durability::Data).unwrap().get(), g + 1);
    let disk = db.into_device().unwrap();
    assert!(matches!(disk.ops().first(), Some(SimOp::Publish { how: PublishHow::Synced, .. })), "{:?}", disk.ops().first());
    assert_eq!(disk.ops().iter().filter(|o| **o == SimOp::Barrier).count(), 2);
    assert_eq!(replay(&Store { cells: disk.cache() }), applied(&replay(&base), &[Op::Put("b", b"2")]));
}
