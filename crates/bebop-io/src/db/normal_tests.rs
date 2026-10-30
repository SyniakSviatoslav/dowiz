//! W-ES-P11: `Durability::Normal` through `Db` -- staged and published at once, NOTHING on the
//! device until the checkpoint, which is one group under two barriers; a power cut opens the
//! last checkpoint with its exact map; the flip keeps the snapshot the writer's image.

use super::*;
use crate::sim::crash::{Model, Tear};
use crate::sim::harness::{run, Opts};
use crate::sim::testkit::{kv_image, next, replay, Map, CAP};
use crate::sim::{SimDisk, SimOp};
use crate::{Detached, Durable};

const POWER: Opts = Opts { model: Model::Power(Tear::Half), pick_only: false, old_intact: true };

fn opts(every: usize) -> Options {
    Options::default().compaction(bebop_kv::delta::MAX_DELTAS, 0).checkpoint_every(every)
}

fn put(db: &Db<SimDisk>, k: &[u8], v: &[u8], d: Durability) -> Generation {
    let mut w = db.begin_write().unwrap();
    w.put(k, v).unwrap();
    w.commit(d).unwrap()
}

fn barriers(ops: &[SimOp]) -> usize {
    ops.iter().filter(|o| **o == SimOp::Barrier).count()
}

fn map_with(m: &Map, kvs: &[(&str, &str)]) -> Map {
    let mut m = m.clone().unwrap_or_default();
    for (k, v) in kvs {
        m.insert(k.to_string(), v.as_bytes().to_vec());
    }
    Some(m)
}

/// Seven `Normal` commits with `checkpoint_every` = 4: visible at once; the device gets nothing
/// until the 4th, which is one group (2 barriers, generation k+4 durable); commits 5-7 write
/// nothing; every power-loss state of the record opens k or k+4 with the exact map; a reopen
/// from the cache (process crash) holds k+4 and loses 5-7 -- never a torn state.
#[test]
fn normal_commits_reach_the_device_only_at_the_checkpoint_as_one_group() {
    let base = kv_image(&[("a", "1")], CAP);
    let m0 = replay(&base);
    let db = Db::open(SimDisk::new(base.cells.clone()), opts(4)).unwrap();
    let k = db.durable().get();
    let kvs = [("b", "2"), ("c", "3"), ("d", "4"), ("e", "5"), ("f", "6"), ("g", "7"), ("h", "8")];
    for (i, (key, v)) in kvs.iter().enumerate() {
        let g = put(&db, key.as_bytes(), v.as_bytes(), Durability::Normal);
        assert_eq!(g.get(), k + 1 + i as i64, "visible generation");
        assert_eq!(db.begin_read().get(key.as_bytes()).unwrap().unwrap(), v.as_bytes(), "read-your-commit");
        assert_eq!(db.snapshot_bytes(), db.writer_image(), "snapshot == writer image after commit {i}");
        let want_durable = if i >= 3 { k + 4 } else { k };
        assert_eq!(db.durable().get(), want_durable, "durable after commit {i}");
        assert_eq!(db.unsynced(), (i as u64 + 1) % 4, "unsynced after commit {i}");
    }
    let disk = db.into_device();
    assert_eq!(barriers(disk.ops()), 2, "one group = two barriers: {:?}", disk.ops().iter().filter(|o| **o == SimOp::Barrier).count());
    let expect = [(k, m0.clone()), (k + 4, map_with(&m0, &kvs[..4]))];
    let r = run(&base.cells, disk.ops(), POWER, &expect, &replay, &next);
    assert!(r.green(), "{} of {} states fail; first {:?}", r.failures.len(), r.states, r.first());
    eprintln!("P11 Normal checkpoint_every=4, 7 commits: {} power-loss states open k or k+4, 2 barriers", r.states);
    let again = Db::open(SimDisk::new(disk.cache()), opts(4)).unwrap();
    assert_eq!(again.begin_read().generation().get(), k + 4, "the process crash keeps the checkpoint only");
    assert_eq!(replay(&Store { cells: disk.cache() }), expect[1].1);
    assert!(again.begin_read().get(b"e").unwrap().is_some(), "commit 4 is the checkpoint");
    assert!(again.begin_read().get(b"f").unwrap().is_none(), "commit 5 was never on the device");
}

/// `Db::flush` and a `Data` commit are checkpoints: everything staged lands as one group.
#[test]
fn an_explicit_flush_or_a_data_commit_checkpoints_the_staged_tail() {
    let base = kv_image(&[("a", "1")], CAP);
    let db = Db::open(SimDisk::new(base.cells.clone()), opts(0)).unwrap();
    let k = db.durable().get();
    for i in 0..5u8 {
        put(&db, &[b'n', i], &[i], Durability::Normal);
    }
    assert_eq!((db.durable().get(), db.unsynced()), (k, 5), "checkpoint_every = 0: never automatic");
    assert_eq!(db.flush(Durability::Data).unwrap().get(), k + 5);
    assert_eq!((db.durable().get(), db.unsynced()), (k + 5, 0));
    assert_eq!(db.snapshot_bytes(), db.writer_image(), "the flip's pages are re-published");
    for i in 0..3u8 {
        put(&db, &[b'm', i], &[i], Durability::Normal);
    }
    assert_eq!(put(&db, b"z", b"z", Durability::Data).get(), k + 9, "a Data commit after 3 Normal ones");
    assert_eq!((db.durable().get(), db.unsynced()), (k + 9, 0));
    assert_eq!(db.snapshot_bytes(), db.writer_image());
    let disk = db.into_device();
    assert_eq!(barriers(disk.ops()), 4, "two groups (N=5 and N=4), two barriers each");
    let again = Db::open(SimDisk::new(disk.cache()), opts(0)).unwrap();
    assert_eq!(again.begin_read().generation().get(), k + 9);
}

/// A device that promises nothing (`Detached`) refuses `Normal` by name, as it refuses `Data`.
#[test]
fn normal_is_refused_where_the_device_cannot_checkpoint() {
    let db = Db::create(Detached, opts(4)).unwrap();
    let mut w = db.begin_write().unwrap();
    w.put(b"a", b"1").unwrap();
    assert!(matches!(w.commit(Durability::Normal), Err(Refusal::Durability { asked: Durability::Normal, device: Durability::None })));
    assert!(Durability::None < Durability::Normal && Durability::Normal < Durability::Data);
}

/// Even and odd groups through `Db` (`stage` + `flush`): two barriers each, the old page untouched,
/// the snapshot the writer's image, and a reopen from the cache at k+N.
#[test]
fn staged_groups_of_every_size_flush_under_two_barriers() {
    for n in 1..=8usize {
        let base = kv_image(&[("a", "1")], CAP);
        let db = Db::open(SimDisk::new(base.cells.clone()), opts(0)).unwrap();
        let k = db.durable().get();
        for i in 0..n {
            let mut w = db.begin_write().unwrap();
            w.put(&[b's', i as u8], &[i as u8]).unwrap();
            w.stage().unwrap();
        }
        assert_eq!(db.flush(Durability::Data).unwrap().get(), k + n as i64);
        assert_eq!(db.snapshot_bytes(), db.writer_image(), "N={n}");
        let disk = db.into_device();
        assert_eq!(barriers(disk.ops()), 2, "N={n}");
        let d = Durable::of(&base).unwrap();
        let fin = disk.cache();
        assert_eq!(fin[d.at..d.at + 39], base.cells[d.at..d.at + 39], "N={n}: durable page untouched");
        assert_eq!(Db::open(SimDisk::new(fin), opts(0)).unwrap().begin_read().generation().get(), k + n as i64);
    }
}

#[cfg(feature = "tree")]
#[test]
fn normal_on_a_v4_image_checkpoints_as_one_group() {
    // `tree(_, 0)`: no automatic compaction (`dead_over` is false at dead_div 0); on a tiny
    // image the 4th path copy would otherwise compact, and a compaction IS a checkpoint.
    let o = opts(3).create_tree(true).tree(bebop_kv::tree::FANOUT, 0);
    let db = Db::create(SimDisk::new(Vec::new()), o).unwrap();
    let mut seeded = db.into_device();
    seeded.settle();
    let db = Db::open(seeded, o).unwrap();
    assert!(db.is_tree());
    let k = db.durable().get();
    for i in 0..5u8 {
        put(&db, &[b't', i], &[i], Durability::Normal);
    }
    assert_eq!((db.durable().get(), db.unsynced()), (k + 3, 2));
    assert_eq!(db.snapshot_bytes(), db.writer_image());
    let disk = db.into_device();
    assert_eq!(barriers(disk.ops()), 2);
    let again = Db::open(SimDisk::new(disk.cache()), opts(3)).unwrap();
    assert_eq!(again.begin_read().generation().get(), k + 3);
    assert!(again.begin_read().get(&[b't', 2]).unwrap().is_some());
    assert!(again.begin_read().get(&[b't', 3]).unwrap().is_none());
}

/// Wall time of 1000 one-put commits on a real file: `Data` (2 barriers each), `Normal`
/// (checkpoint every 64: 2 barriers per 64), `None` (writes, no barrier). Run by hand:
/// `P11_DIR=<dir> cargo test --lib normal_tests::timing -- --ignored --nocapture`.
#[test]
#[ignore]
fn timing() {
    let dir = std::env::var("P11_DIR").expect("P11_DIR");
    for (name, d) in [("Data", Durability::Data), ("Normal/64", Durability::Normal), ("None", Durability::None)] {
        let path = format!("{dir}/p11-{}.bebop", name.replace('/', "-"));
        let _ = std::fs::remove_file(&path);
        let db = Db::create(crate::FileDevice::create(&path, 1 << 20).unwrap(), Options::default().checkpoint_every(64)).unwrap();
        let t = std::time::Instant::now();
        for i in 0..1000u32 {
            let mut w = db.begin_write().unwrap();
            w.put(&i.to_be_bytes(), &[7u8; 64]).unwrap();
            w.commit(d).unwrap();
        }
        if d == Durability::Normal {
            db.flush(Durability::Data).unwrap();
        }
        let el = t.elapsed();
        println!("P11 timing {name}: 1000 commits in {:.1} ms = {:.1} us/commit (compactions {})", el.as_secs_f64() * 1e3, el.as_secs_f64() * 1e6 / 1000.0, db.compactions());
    }
}
