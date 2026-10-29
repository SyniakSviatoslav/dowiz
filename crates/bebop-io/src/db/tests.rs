//! W-ES-P3: T-ES-10 (isolation), T-ES-11 (one writer), T-ES-15 (compaction), T-ES-16 (limits),
//! over `SimDisk` (power semantics), `Detached` (dowiz) and, for T-ES-11b/T-ES-15c, a real file.
//!
//! THE RED MUTANTS (each applied by hand, run, shown failing with the assertion quoted, reverted;
//! see the lane hand-back for the runs):
//!   M10a  `begin_read` reads the writer's live image instead of the published Arc
//!         -> t_es_10: r1 reads a=2 after the commit.
//!   M10b  `WriteTxn::get` skips the batch -> t_es_10: w.get(a) = 1.
//!   M10c  the snapshot is published BEFORE `flush` and kept on Err
//!         -> t_es_10_a_failed_barrier_publishes_nothing: a reader sees k+1 after Poisoned.
//!   M11a  `begin_write` takes a fresh lock per call -> t_es_11_in_process: the second is Ok.
//!   M11b  `FileDevice::open` skips `try_lock` -> t_es_11_cross_process: the child opens it.
//!   M11c  no pid written into the lock file -> t_es_11_cross_process: `by == None`.
//!   M15a  compaction writes the base only (no chain, no batch) -> t_es_15: the fold differs.
//!   M15b  compaction mutates the published snapshot in place -> t_es_15: r_old's generation moves.
//!   M15c  compaction restarts at generation 1 (`after_gen = None`) -> t_es_15: gen != g + 1.
//!   M16b  `WriteTxn::put` skips the limits -> t_es_16_put: the edge + 1 is accepted.
//!   M16c  `Store::begin`'s generation check removed -> t_es_16_generation: the commit past
//!         MAX_GENERATION is accepted. (M16a, `>` -> `>=`, is bebop-kv's limits/tests.rs.)
use std::ops::Bound::Unbounded;

use super::*;
use bebop_kv::Limits;
use crate::sim::SimDisk;
use crate::Detached;

type Entries = Vec<(Vec<u8>, Vec<u8>)>;

fn all<D: Device>(db: &Db<D>) -> Entries {
    let r = db.begin_read();
    let out = r.scan(Unbounded, Unbounded).unwrap().map(|e| e.map(|(k, v)| (k.into_owned(), v.into_owned())).unwrap()).collect();
    out
}

/// The published snapshot is the writer's image, byte for byte.
fn snapshot_is_the_writer<D: Device>(db: &Db<D>) {
    assert_eq!(db.snapshot_bytes(), db.writer_image(), "the published snapshot is not the writer's trimmed image");
}

/// The delta writer's options with the dead-cells trigger off (`dead_div` 0): on images this
/// small every few edits would otherwise compact (superseded > live / 4), and these tests are
/// about deltas, barriers and generations, which a compaction would change under them.
fn deltas_only() -> Options {
    Options::default().compaction(bebop_kv::delta::MAX_DELTAS, 0)
}

fn seeded<D: Device>(dev: D, d: Durability) -> Db<D> {
    let db = Db::create(dev, deltas_only()).unwrap();
    let mut w = db.begin_write().unwrap();
    w.put(b"a", b"1").unwrap();
    w.put(b"c", b"3").unwrap();
    w.commit(d).unwrap();
    snapshot_is_the_writer(&db);
    db
}

fn t_es_10_on<D: Device>(db: Db<D>, d: Durability) {
    let k = db.begin_read().generation();
    let r1 = db.begin_read();
    let r1_bytes = r1.image().to_vec();
    let mut w = db.begin_write().unwrap();
    w.put(b"a", b"2").unwrap();
    w.put(b"b", b"2").unwrap();
    assert!(w.remove(b"c").unwrap(), "c was visible to the txn");
    assert!(!w.remove(b"z").unwrap(), "z never was");
    assert_eq!(w.get(b"a").unwrap().unwrap(), &b"2"[..], "read-your-writes");
    let seen: Entries = w.scan(Unbounded, Unbounded).unwrap().map(|e| e.map(|(k, v)| (k.into_owned(), v.into_owned())).unwrap()).collect();
    assert_eq!(seen, vec![(b"a".to_vec(), b"2".to_vec()), (b"b".to_vec(), b"2".to_vec())]);
    assert_eq!(r1.get(b"a").unwrap().unwrap(), &b"1"[..]);
    assert_eq!(r1.get(b"b").unwrap(), None);
    assert_eq!(r1.get(b"c").unwrap().unwrap(), &b"3"[..]);
    let g = w.commit(d).unwrap();
    assert_eq!(g.get(), k.get() + 1);
    assert_eq!(r1.generation(), k, "a reader's generation moved under it");
    let a = r1.get(b"a").unwrap().unwrap();
    assert_eq!(a, &b"1"[..], "a reader saw the commit");
    assert!(a.is_borrowed(), "a v2/v3 value is read in place");
    assert_eq!(r1.image(), &r1_bytes[..], "a reader's bytes changed under it");
    let r2 = db.begin_read();
    assert_eq!(r2.generation(), g);
    assert_eq!(all(&db), vec![(b"a".to_vec(), b"2".to_vec()), (b"b".to_vec(), b"2".to_vec())]);
    snapshot_is_the_writer(&db);
    // abort: nothing touched the image (AC-ES-12)
    let before = db.snapshot_bytes();
    let mut w2 = db.begin_write().unwrap();
    w2.put(b"x", b"9").unwrap();
    w2.abort();
    assert_eq!(db.snapshot_bytes(), before);
    assert_eq!(db.begin_read().get(b"x").unwrap(), None);
    assert_eq!(db.begin_read().generation(), g);
}

#[test]
fn t_es_10_isolation_over_a_simulated_disk() {
    t_es_10_on(seeded(SimDisk::new(Vec::new()), Durability::Data), Durability::Data);
}

#[test]
fn t_es_10_isolation_detached() {
    t_es_10_on(seeded(Detached, Durability::None), Durability::None);
}

#[test]
fn t_es_10_a_failed_barrier_publishes_nothing_and_poisons() {
    // create = 3 barriers (0: the formatted image, 1-2: the empty schema's commit), the seed
    // commit = 2 (3, 4): the next commit's first barrier is number 5.
    let db = seeded(SimDisk::new(Vec::new()).failing_barrier(5), Durability::Data);
    let k = db.begin_read().generation();
    let mut w = db.begin_write().unwrap();
    w.put(b"a", b"lost").unwrap();
    let e = w.commit(Durability::Data).unwrap_err();
    assert!(matches!(e, Refusal::Io { os: Some(5), .. }), "{e:?}");
    assert_eq!(db.begin_read().generation(), k, "a commit that failed was published");
    assert_eq!(db.begin_read().get(b"a").unwrap().unwrap(), &b"1"[..]);
    assert!(db.poisoned());
    assert!(matches!(db.begin_write(), Err(Refusal::Poisoned)));
    let db = db.reopen().unwrap();
    assert_eq!(db.begin_read().generation(), k, "the device holds generation k");
    assert!(db.begin_write().is_ok());
}

#[test]
fn a_detached_db_refuses_more_durability_than_it_has() {
    let db = seeded(Detached, Durability::None);
    let mut w = db.begin_write().unwrap();
    w.put(b"q", b"1").unwrap();
    let e = w.commit(Durability::Data).unwrap_err();
    assert_eq!(e, Refusal::Durability { asked: Durability::Data, device: Durability::None });
}

#[test]
fn an_empty_commit_makes_no_generation() {
    let db = seeded(Detached, Durability::None);
    let g = db.begin_read().generation();
    let before = db.snapshot_bytes();
    assert_eq!(db.begin_write().unwrap().commit(Durability::None).unwrap(), g);
    assert_eq!(db.snapshot_bytes(), before);
}

#[test]
fn staged_commits_are_visible_at_once_and_durable_after_flush() {
    let db = seeded(SimDisk::new(Vec::new()), Durability::Data);
    let d0 = db.durable();
    for i in 0..3u8 {
        let mut w = db.begin_write().unwrap();
        w.put(&[b'g', i], &[i]).unwrap();
        w.stage().unwrap();
    }
    assert_eq!(db.begin_read().generation().get(), d0.get() + 3, "visible");
    assert_eq!(db.durable(), d0, "not yet durable");
    snapshot_is_the_writer(&db);
    assert_eq!(db.flush(Durability::Data).unwrap().get(), d0.get() + 3);
    assert_eq!(db.durable().get(), d0.get() + 3);
    let dev = db.into_device();
    let again = Db::open(dev, Options::default()).unwrap();
    assert_eq!(again.begin_read().get(&[b'g', 2]).unwrap().unwrap(), &[2u8][..]);
}

// ── T-ES-11 ──

#[test]
fn t_es_11_in_process_a_second_writer_is_busy_and_a_panic_poisons() {
    let db = seeded(Detached, Durability::None);
    let w1 = db.begin_write().unwrap();
    assert!(matches!(db.begin_write(), Err(Refusal::WriterBusy)));
    drop(w1);
    assert!(db.begin_write().is_ok());
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _w = db.begin_write().unwrap();
        panic!("a panic inside a write transaction");
    }));
    assert!(r.is_err());
    assert!(matches!(db.begin_write(), Err(Refusal::Poisoned)));
    assert_eq!(db.begin_read().get(b"a").unwrap().unwrap(), &b"1"[..], "reads go on");
    let db = db.reopen().unwrap();
    assert!(db.begin_write().is_ok());
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
fn tmp_store(tag: &str) -> String {
    let p = std::env::temp_dir().join(format!("bebop-p3-{tag}-{}.store", std::process::id()));
    let p = p.to_string_lossy().into_owned();
    for f in [p.clone(), crate::file::lock_path(&p), crate::file::compact_path(&p)] {
        let _ = std::fs::remove_file(f);
    }
    p
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn t_es_11_cross_process_a_second_process_is_locked_out_with_the_holders_pid() {
    let p = tmp_store("t11");
    let db = Db::create_path(&p, Options::default()).unwrap();
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "db::tests::t_es_11_child", "--ignored", "--nocapture", "--test-threads=1"])
        .env("BEBOP_T11", &p)
        .env("BEBOP_T11_PARENT", std::process::id().to_string())
        .output()
        .unwrap();
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success(), "the child failed: {text}");
    // A child that ran ZERO tests also exits 0: a skip is not a pass.
    assert!(text.contains("1 passed"), "the child did not run the test: {text}");
    drop(db);
    let _ = std::fs::remove_file(&p);
    let _ = std::fs::remove_file(crate::file::lock_path(&p));
}

/// Run only by `t_es_11_cross_process...` in a child process (it sets BEBOP_T11).
#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
#[ignore = "run in a child process by t_es_11_cross_process"]
fn t_es_11_child() {
    let (Ok(p), Ok(parent)) = (std::env::var("BEBOP_T11"), std::env::var("BEBOP_T11_PARENT")) else {
        panic!("t_es_11_child runs only as the child of t_es_11_cross_process (BEBOP_T11 unset)");
    };
    let parent: u32 = parent.parse().unwrap();
    match Db::open_path(&p, Options::default()) {
        Err(Refusal::Locked { by }) => assert_eq!(by, Some(parent), "the lock names its holder"),
        Err(e) => panic!("expected Locked, got {e:?}"),
        Ok(_) => panic!("a second process opened a locked store"),
    }
}

// ── T-ES-15 ──

/// A v3 image: 10 base entries, then 7 delta ops in 3 commits, one of them a remove.
fn v3<D: Device>(db: &Db<D>, d: Durability) {
    let mut w = db.begin_write().unwrap();
    for i in 0..10u8 {
        w.put(format!("k{i:02}").as_bytes(), format!("base {i}").as_bytes()).unwrap();
    }
    w.commit_compacted(d).unwrap();
    for ops in [&[("k01", Some("one")), ("k02", Some("two")), ("k20", Some("new"))][..], &[("k03", Some("three")), ("k05", None), ("k06", Some("six"))], &[("k07", Some("seven"))]] {
        let mut w = db.begin_write().unwrap();
        for (k, v) in ops {
            match v {
                Some(v) => w.put(k.as_bytes(), v.as_bytes()).unwrap(),
                None => {
                    w.remove(k.as_bytes()).unwrap();
                }
            }
        }
        w.commit(d).unwrap();
    }
    assert_eq!(bebop_kv::Kv::version(&Store::from_bytes(&db.snapshot_bytes())), 3, "the fixture is not v3");
}

fn t_es_15_on<D: Device>(db: Db<D>, d: Durability, compact: impl FnOnce(&Db<D>) -> Result<Generation, Refusal>) {
    v3(&db, d);
    let r_old = db.begin_read();
    let (fold0, len0, keys0, g) = (r_old.fold_root().unwrap(), r_old.len(), all(&db), r_old.generation());
    let g2 = compact(&db).unwrap();
    assert_eq!(g2.get(), g.get() + 1, "a compaction moved the generation other than by one");
    let r_new = db.begin_read();
    assert_eq!(r_new.generation(), g2);
    assert_eq!(r_new.fold_root().unwrap(), fold0, "the compaction changed the fold");
    assert_eq!(r_new.len(), len0);
    assert_eq!(all(&db), keys0, "the compaction changed the entries or their order");
    assert_eq!(bebop_kv::Kv::version(&Store::from_bytes(r_new.image())), 2, "a compaction writes v2");
    assert_eq!(r_old.generation(), g, "the old reader's generation moved");
    assert_eq!(r_old.fold_root().unwrap(), fold0);
    for (k, v) in &keys0 {
        assert_eq!(r_old.get(k).unwrap().unwrap(), &v[..], "the old reader lost {k:?}");
    }
    assert_eq!(db.durable(), g2);
    snapshot_is_the_writer(&db);
}

#[test]
fn t_es_15_compaction_into_a_new_disk_keeps_the_entries_and_the_old_reader() {
    t_es_15_on(Db::create(SimDisk::new(Vec::new()), Options::default()).unwrap(), Durability::Data, |db| db.compact(SimDisk::new(Vec::new())));
}

#[test]
fn t_es_15_compaction_detached() {
    t_es_15_on(Db::create_detached(Options::detached()).unwrap(), Durability::None, |db| db.compact(Detached));
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
#[test]
fn t_es_15_compaction_of_a_file_publishes_by_rename() {
    let p = tmp_store("t15");
    let db = Db::create_path(&p, Options::default()).unwrap();
    t_es_15_on(db, Durability::Data, |db| db.compact_fresh());
    assert!(!std::path::Path::new(&crate::file::compact_path(&p)).exists(), "the staged file was not renamed over the store");
    let again = Db::open_path(&p, Options::default().verify(Verify::WholeArena));
    // The first Db still holds the lock until it is dropped inside t_es_15_on.
    let again = again.unwrap();
    assert_eq!(again.begin_read().get(b"k07").unwrap().unwrap(), &b"seven"[..]);
    drop(again);
    let _ = std::fs::remove_file(&p);
    let _ = std::fs::remove_file(crate::file::lock_path(&p));
}

#[test]
fn t_es_15_the_delta_writer_compacts_on_its_own_trigger_and_the_generation_only_climbs() {
    let db = Db::create(SimDisk::new(Vec::new()), Options::default().compaction(4, bebop_kv::delta::DEAD_DIV)).unwrap();
    let mut want = bebop_kv::Kv { entries: Vec::new() };
    let mut last = db.begin_read().generation();
    let mut saw_v2_after_v3 = false;
    for i in 0..12u8 {
        let mut w = db.begin_write().unwrap();
        let k = format!("key{}", i % 5);
        w.put(k.as_bytes(), &[i]).unwrap();
        want.put(&k, &[i]);
        let g = w.commit(Durability::Data).unwrap();
        assert_eq!(g.get(), last.get() + 1);
        last = g;
        let ver = bebop_kv::Kv::version(&Store::from_bytes(&db.snapshot_bytes()));
        saw_v2_after_v3 |= ver == 2 && i > 0;
        assert_eq!(db.begin_read().fold_root().unwrap(), want.snapshot_root_u64(), "after commit {i}");
        snapshot_is_the_writer(&db);
    }
    assert!(saw_v2_after_v3, "the trigger never compacted");
}

#[test]
fn verify_proves_each_depth() {
    let db = seeded(SimDisk::new(Vec::new()), Durability::Data);
    v3(&db, Durability::Data);
    let a = db.verify(Verify::Anchor).unwrap();
    assert!(matches!(a.via, crate::sim::Via::Anchor { .. }), "{a:?}");
    let w = db.verify(Verify::WholeArena).unwrap();
    assert!(w.objects > 0);
    assert_eq!(w.generation, db.begin_read().generation());
}

// ── T-ES-16 ──

#[test]
fn t_es_16_put_refuses_past_the_limits_at_put_and_changes_nothing() {
    let db = Db::create_detached(Options::detached().limits(Limits { key: 64, value: 1024 })).unwrap();
    let mut w = db.begin_write().unwrap();
    w.put(&[b'k'; 64], &[0; 1024]).unwrap();
    assert_eq!(w.put(&[b'k'; 65], b"v"), Err(Refusal::KeyTooLong { len: 65, max: 64 }));
    assert_eq!(w.put(b"v", &[0; 1025]), Err(Refusal::ValueTooLong { len: 1025, max: 1024 }));
    assert_eq!(w.len().unwrap(), 1, "a refused put changed the transaction");
    assert_eq!(w.get(b"v").unwrap(), None);
    w.commit(Durability::None).unwrap();
    // a batch made under looser limits is checked when it is handed in
    let mut loose = Batch::default();
    loose.put(&[b'x'; 100], b"v").unwrap();
    assert!(matches!(db.begin_write_with(loose), Err(Refusal::KeyTooLong { len: 100, max: 64 })));
}

#[test]
fn t_es_16_the_image_ceiling_accepts_the_last_cell_and_refuses_one_past_it() {
    // A 32 KiB image has 4096 - 1024 = 3072 arena cells. One entry, key "v" (1 byte), value L:
    // KIDX 2+2, KBLOB 1+2, VIDX 2+2, VBLOB ceil(L/8)+2, root 6+2 = 21 + ceil(L/8) cells.
    // So L = (3072 - 21) * 8 = 24 408 fits exactly; one byte more needs one more cell.
    let edge = (3072 - 21) * 8;
    for (len, fits) in [(edge, true), (edge + 1, false)] {
        let db = Db::create_detached(Options::detached().max_image_bytes(32 * 1024)).unwrap();
        let mut w = db.begin_write().unwrap();
        w.put(b"v", &vec![7u8; len]).unwrap();
        let r = w.commit_compacted(Durability::None);
        match (fits, r) {
            (true, Ok(_)) => {}
            (false, Err(Refusal::ArenaFull { .. })) => {}
            (f, r) => panic!("value of {len} bytes: expected fits={f}, got {r:?}"),
        }
    }
}

#[test]
fn t_es_16_the_generation_limit_refuses_the_commit_past_it() {
    let entries = [(&b"a"[..], &b"1"[..])];
    // committed as generation MAX - 1
    let bytes = bebop_kv::compacted_from(entries.iter().copied(), 16 * 1024, Some(crate::MAX_GENERATION - 2)).unwrap();
    let db = Db::from_bytes(&bytes, Options::detached()).unwrap();
    assert_eq!(db.begin_read().generation().get(), crate::MAX_GENERATION - 1);
    let mut w = db.begin_write().unwrap();
    w.put(b"b", b"2").unwrap();
    assert_eq!(w.commit(Durability::None).unwrap().get(), crate::MAX_GENERATION, "the last generation is accepted");
    let mut w = db.begin_write().unwrap();
    w.put(b"c", b"3").unwrap();
    let e = w.commit(Durability::None).unwrap_err();
    assert!(matches!(e, Refusal::Limit { what: "generation", .. }), "{e:?}");
    assert_eq!(db.begin_read().generation().get(), crate::MAX_GENERATION);
}

// ── adopted bytes (dowiz) ──

#[test]
fn a_torn_last_commit_in_adopted_bytes_is_refused_not_fallen_back_past() {
    let db = seeded(Detached, Durability::None);
    let mut w = db.begin_write().unwrap();
    w.put(b"a", b"newest").unwrap();
    w.commit(Durability::None).unwrap();
    let mut bytes = db.snapshot_bytes();
    // one changed byte in the last cell of the last commit: the anchor no longer holds
    let n = bytes.len();
    bytes[n - 1] ^= 0x01;
    let e = Db::from_bytes(&bytes, Options::detached()).err().expect("a torn image was adopted");
    // The anchor says only that the last commit changed; the refusal names the object in it
    // whose payload no longer hashes to its crc (the newest root: the image's last object).
    let st = Store::from_bytes(&db.snapshot_bytes());
    let root = st.root().unwrap();
    assert!(matches!(e, Refusal::BadCrc(b) if b.obj == root), "{e:?} (root {root})");
    // the file path's rule is the fallback: generation k
    let fell = Db::from_bytes(&bytes, Options::detached().on_torn(OnTorn::Fallback)).unwrap();
    assert_eq!(fell.begin_read().get(b"a").unwrap().unwrap(), &b"1"[..]);
}
