//! W-ES-P8C: `Db` over a v4 image -- the tree commit, the dead-ratio compaction (and ONLY it),
//! the explicit migration, reopen. Every expectation is a `BTreeMap` with the ops applied by hand.

use std::collections::BTreeMap;
use std::ops::Bound::Unbounded;

use super::*;
use crate::sim::SimDisk;
use crate::Detached;

type Model = BTreeMap<Vec<u8>, Vec<u8>>;

fn key(i: usize) -> Vec<u8> {
    format!("k{i:05}").into_bytes()
}

fn all<D: Device>(db: &Db<D>) -> Model {
    let r = db.begin_read();
    let out = r.scan(Unbounded, Unbounded).unwrap().map(|e| e.map(|(k, v)| (k.into_owned(), v.into_owned())).unwrap()).collect();
    out
}

/// One commit of puts `ps` and removes `rs`, applied to `m` by hand too.
fn commit<D: Device>(db: &Db<D>, m: &mut Model, ps: &[usize], rs: &[usize], g: usize, d: Durability) {
    let mut w = db.begin_write().unwrap();
    for &i in ps {
        let v = format!("{g}:{i}").into_bytes();
        w.put(&key(i), &v).unwrap();
        m.insert(key(i), v);
    }
    for &i in rs {
        assert_eq!(w.remove(&key(i)).unwrap(), m.remove(&key(i)).is_some());
    }
    w.commit(d).unwrap();
}

fn check<D: Device>(db: &Db<D>, m: &Model) {
    assert!(db.is_tree(), "the image stays v4");
    assert_eq!(&all(db), m);
    assert_eq!(db.snapshot_bytes(), db.writer_image(), "the published snapshot is the writer's image");
    db.verify(Verify::WholeArena).unwrap();
    let r = db.begin_read();
    assert_eq!(r.len(), m.len());
    for (k, v) in m.iter().take(50) {
        assert_eq!(r.get(k).unwrap().as_deref(), Some(&v[..]));
    }
}

/// 300 commits on a 500-key v4 map: every one a path copy (no compaction) until the dead cells
/// pass the live ones, and then the copy compaction, generation always + 1.
#[test]
fn a_v4_db_commits_by_path_copy_and_compacts_only_on_the_dead_ratio() {
    let db = Db::create_detached(Options::detached().create_tree(true)).unwrap();
    let mut m = Model::new();
    commit(&db, &mut m, &(0..500).collect::<Vec<_>>(), &[], 0, Durability::None);
    check(&db, &m);
    let mut gen = db.begin_read().generation().get();
    let mut first = None;
    for g in 1..300usize {
        let before = db.compactions();
        commit(&db, &mut m, &[g * 7 % 600, g * 13 % 600], &[g * 11 % 500], g, Durability::None);
        let now = db.begin_read().generation().get();
        assert_eq!(now, gen + 1, "commit {g}: the generation climbs by one");
        gen = now;
        if db.compactions() > before {
            first.get_or_insert(g);
            let sb = db.begin_read().superblock();
            assert_eq!(sb.superseded_cells, 0, "commit {g}: a compaction leaves no dead cells");
        } else {
            let sb = db.begin_read().superblock();
            assert!(sb.superseded_cells <= sb.live_cells + 4000, "commit {g}: dead {} live {}", sb.superseded_cells, sb.live_cells);
        }
    }
    check(&db, &m);
    let first = first.expect("the dead ratio must trigger within 300 commits");
    assert!(first > 5, "a fresh image must take several commits to pass the ratio: {first}");
    eprintln!("P8c Db v4: first compaction at commit {first}, {} in 300", db.compactions());
}

/// MIGRATION is explicit: a v2/v3 `Db` stays v2/v3 across commits and reopen; `migrate_to_tree`
/// makes it v4 with the same entries, generation + 1, and later commits are tree commits.
#[test]
fn migration_is_explicit_and_keeps_the_map() {
    let db = Db::create(SimDisk::new(Vec::new()), Options::default().compaction(usize::MAX, 0)).unwrap();
    let mut m = Model::new();
    commit(&db, &mut m, &(0..100).collect::<Vec<_>>(), &[], 0, Durability::Data);
    commit(&db, &mut m, &[5, 200], &[7], 1, Durability::Data);
    assert!(!db.is_tree());
    let db = db.reopen().unwrap();
    assert!(!db.is_tree(), "open never migrates");
    let g = db.begin_read().generation().get();
    assert_eq!(db.migrate_to_tree_fresh().unwrap().get(), g + 1);
    check(&db, &m);
    commit(&db, &mut m, &[1, 300], &[2], 2, Durability::Data);
    check(&db, &m);
    let db = db.reopen().unwrap();
    check(&db, &m);
    assert_eq!(db.begin_read().fold_root().unwrap(), bebop_kv::Kv { entries: m.iter().map(|(k, v)| (String::from_utf8(k.clone()).unwrap(), v.clone())).collect() }.snapshot_root_u64());
}

/// A tree commit through `Writer` on a v2 image is refused by name, the image untouched.
#[test]
fn a_tree_commit_on_a_v2_image_is_refused() {
    let db = Db::create_detached(Options::detached()).unwrap();
    let before = db.snapshot_bytes();
    let mut w = Writer::adopt(Detached, Store::from_bytes(&before), false).unwrap();
    let ops = [bebop_kv::delta::KeyOp::Put(b"a", b"1")];
    assert!(matches!(w.stage_tree(&ops, bebop_kv::tree::FANOUT), Err(crate::IoError::Tree(bebop_kv::tree::WriteError::NotTree { v: 2 }))));
    assert_eq!(w.store().to_bytes_trimmed(), before);
}
