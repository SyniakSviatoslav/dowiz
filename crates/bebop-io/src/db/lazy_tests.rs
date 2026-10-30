//! W-ES-P10 (R-EMBED-STORE §12 P10, FORMAT §7 phase P2): THE DOCUMENTED GUARANTEE OF THE LAZY OPEN.
//!
//! An open at `Verify::Anchor` (the default) proves the pick, the PartTab's crc and the anchor over
//! the LAST commit, and nothing older: O(last commit). So a corrupted object of an OLDER commit is
//! NOT caught by `open` -- by design, stated here, not by accident -- and IS caught by
//! `verify(Reachable)` when the KV reaches it and by `verify(WholeArena)` always. The last commit
//! itself is under one rule on every reader (`bebop_image::verify::last_commit_in`): over bytes
//! handed whole a torn anchor names its failing object (`BadCrc`) or itself (`LastCommit`), and is
//! never fallen back past.
//!
//! RED MUTANTS (applied by hand, run, reverted; the lane hand-back quotes the runs):
//!   M-P10a  `Snap::build` opens with `KvIn::open` at `Verify::Anchor` too
//!           -> `an_old_commits_corruption_is_not_caught_by_open`: the open refuses (BadCrc).
//!   M-P10b  `Db::verify` skips `KvIn::open` for `Reachable`
//!           -> the same test: verify(Reachable) is Ok on the corrupted image.
//!   M-P10c  `last_commit_in` returns Ok on Torn -> `the_last_commit_rule_*`: no LastCommit refusal.
//!   M-P10d  `Db::open` builds the writer eagerly (`Lazy::Ready(writer_of(dev)?)`)
//!           -> `open_reads_the_live_image_once_and_the_writer_only_at_the_first_write`: a second
//!           read of the whole device before any write.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::sim::Via;
use crate::Detached;
use bebop_image::verify::BadCrc;
use bebop_image::View;
use bebop_kv::verify::KvError;
use bebop_kv::zc::KvIn;

/// A v3 image over bytes: a compacted base of 10 keys and three delta commits (four commits in
/// the arena; the last one's record and root are the anchor's range).
fn v3_bytes() -> Vec<u8> {
    let db = Db::create(Detached, Options::detached().compaction(bebop_kv::delta::MAX_DELTAS, 0)).unwrap();
    let mut w = db.begin_write().unwrap();
    for i in 0..10u8 {
        w.put(format!("k{i:02}").as_bytes(), format!("base {i}").as_bytes()).unwrap();
    }
    w.commit_compacted(Durability::None).unwrap();
    for ops in [&["k01", "k02", "k20"][..], &["k03", "k05", "k06"], &["k07"]] {
        let mut w = db.begin_write().unwrap();
        for k in ops {
            w.put(k.as_bytes(), format!("new {k}").as_bytes()).unwrap();
        }
        w.commit(Durability::None).unwrap();
    }
    db.snapshot_bytes()
}

/// A v4 (B+tree) image over bytes: six path-copying commits.
#[cfg(feature = "tree")]
fn v4_bytes() -> Vec<u8> {
    // dead_div 0: the copy compaction OFF, so the six commits stay in one arena and the last one's
    // mark is above the first five (a compaction's "last commit" is the whole image).
    let db = Db::create(Detached, Options::detached().create_tree(true).tree(8, 0).initial_bytes(256 * 1024)).unwrap();
    for round in 0..6u8 {
        let mut w = db.begin_write().unwrap();
        for i in (0..60u8).filter(|i| i % 6 == round) {
            w.put(format!("k/{:03}", (i as usize * 37) % 60).as_bytes(), &[i; 9]).unwrap();
        }
        w.commit(Durability::None).unwrap();
    }
    db.snapshot_bytes()
}

/// The anchor's mark: the first cell of the last commit.
fn mark_of(bytes: &[u8]) -> usize {
    let v = View::new(bytes);
    let sb = crate::format::pick_checked_in(&v).unwrap();
    match crate::format::anchor_in(&v, &sb) {
        crate::format::Anchor::Holds { mark, .. } => mark,
        a => panic!("the writer's image must be anchored: {a:?}"),
    }
}

/// Every object below the mark (an OLDER commit's), paired with whether the KV reaches it: a bit
/// flipped in its payload makes the full reader (`KvIn::open`) name exactly it (`Some(true)`),
/// changes nothing it reads (`Some(false)`: superseded, reached by nothing), or trips a structural
/// rule before any crc (`None`: not a witness either way).
fn old_objects(bytes: &[u8]) -> Vec<(usize, Option<bool>)> {
    let mark = mark_of(bytes);
    let v = View::new(bytes);
    let mut out = Vec::new();
    let mut o = ARENA;
    while o < mark {
        let len = bebop_image::obj_cells_in(&v, o);
        if len > 0 {
            let flipped = flip(bytes, o);
            let reached = match KvIn::open(View::new(&flipped)) {
                Ok(_) => Some(false),
                Err(KvError::BadCrc(b)) | Err(KvError::Tree(bebop_kv::tree::TreeError::BadCrc(b))) => {
                    assert_eq!(b.obj, o, "the full reader must name the object that was flipped");
                    Some(true)
                }
                Err(_) => None,
            };
            out.push((o, reached));
        }
        o += 2 + len;
    }
    assert!(out.iter().any(|&(_, r)| r == Some(true)), "no reachable object below the mark {mark}");
    out
}

/// `bytes` with bit 0 of object `o`'s LAST payload cell flipped (a value byte in a record or a
/// blob, so the shape rules still hold and only the crc speaks); the anchor is untouched.
fn flip(bytes: &[u8], o: usize) -> Vec<u8> {
    let mut b = bytes.to_vec();
    let len = bebop_image::obj_cells_in(&View::new(bytes), o);
    b[(o + 2 + len - 1) * 8] ^= 1;
    assert_eq!(mark_of(&b), mark_of(bytes), "the flip must leave the anchor holding");
    b
}

fn check_old_commit(clean: &[u8]) {
    let objs = old_objects(clean);
    let (o, _) = objs.iter().copied().find(|&(_, r)| r == Some(true)).unwrap();
    let bad = flip(clean, o);

    // NOT caught by the lazy open: the anchor covers the last commit only.
    let db = Db::from_bytes(&bad, Options::detached()).expect("open(Anchor) must succeed on an older commit's corruption");
    let _ = db.begin_read().len(); // a wrong answer at worst, never a panic
    let a = db.verify(Verify::Anchor).unwrap();
    assert!(matches!(a.via, Via::Anchor { .. }), "{a:?}");
    // CAUGHT by verify(Reachable) and verify(WholeArena), naming the object.
    assert_eq!(db.verify(Verify::Reachable), Err(Refusal::BadCrc(BadCrc { obj: o, ..bad_crc(&bad, o) })));
    assert!(matches!(db.verify(Verify::WholeArena), Err(Refusal::BadCrc(b)) if b.obj == o));
    // and by an open that asks for that depth
    assert!(matches!(Db::from_bytes(&bad, Options::detached().verify(Verify::Reachable)), Err(Refusal::BadCrc(b)) if b.obj == o));
    assert!(matches!(Db::from_bytes(&bad, Options::detached().verify(Verify::WholeArena)), Err(Refusal::BadCrc(b)) if b.obj == o));

    // POSITIVE TWIN: the clean image opens and verifies at every depth.
    for depth in [Verify::Anchor, Verify::Reachable, Verify::WholeArena] {
        let db = Db::from_bytes(clean, Options::detached().verify(depth)).unwrap();
        let r = db.verify(depth).unwrap();
        assert_eq!(r.depth, depth);
        assert!(matches!(r.via, Via::Anchor { .. }));
    }
}

fn bad_crc(bytes: &[u8], o: usize) -> BadCrc {
    bebop_image::verify::check_obj_in(&View::new(bytes), o).unwrap_err()
}

#[test]
fn an_old_commits_corruption_is_not_caught_by_open_and_is_caught_by_verify() {
    check_old_commit(&v3_bytes());
}

#[cfg(feature = "tree")]
#[test]
fn an_old_commits_corruption_is_not_caught_by_open_and_is_caught_by_verify_v4() {
    check_old_commit(&v4_bytes());
}

#[test]
fn an_unreachable_old_object_is_caught_only_by_whole_arena() {
    // A superseded root of an older generation: in the arena, reached by nothing.
    let clean = v3_bytes();
    let objs = old_objects(&clean);
    let (o, _) = objs.iter().copied().find(|&(_, r)| r == Some(false)).expect("v3 commits leave superseded roots in the arena");
    let bad = flip(&clean, o);
    let db = Db::from_bytes(&bad, Options::detached()).unwrap();
    assert!(db.verify(Verify::Anchor).is_ok());
    assert!(db.verify(Verify::Reachable).is_ok(), "an unreachable object is not Reachable's business");
    assert_eq!(db.verify(Verify::WholeArena), Err(Refusal::ObjectBad { at: o }));
    assert_eq!(Db::from_bytes(&bad, Options::detached().verify(Verify::WholeArena)).err(), Some(Refusal::ObjectBad { at: o }));
    assert!(Db::from_bytes(&bad, Options::detached().verify(Verify::Reachable)).is_ok());
}

/// A device that counts what is read from it, cells and calls, into counters the test holds
/// (shared, so they are read without taking the writer's lock a `WriteTxn` holds).
struct Counted<D: Device> {
    inner: D,
    cells: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
}

impl<D: Device> Device for Counted<D> {
    fn len_cells(&self) -> std::io::Result<usize> {
        self.inner.len_cells()
    }
    fn read_cells(&self, off: usize, n: usize) -> std::io::Result<Vec<i64>> {
        self.cells.fetch_add(n, Ordering::SeqCst);
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.read_cells(off, n)
    }
    fn write_cells(&mut self, off: usize, cells: &[i64]) -> std::io::Result<()> {
        self.inner.write_cells(off, cells)
    }
    fn barrier(&mut self) -> std::io::Result<()> {
        self.inner.barrier()
    }
    fn durability(&self) -> Durability {
        self.inner.durability()
    }
    fn fresh(&self) -> std::io::Result<Self> {
        Ok(Counted { inner: self.inner.fresh()?, cells: Arc::clone(&self.cells), calls: Arc::clone(&self.calls) })
    }
    fn publish(&mut self, staged: Self) -> std::io::Result<()> {
        self.inner.publish(staged.inner)
    }
}

/// THE OPEN IS O(live image) ONCE, AND THE WRITER IS NOT BUILT UNTIL THE FIRST WRITE (W-ES-P10,
/// B4): over a device whose file is padded far past its arena, `open` reads the superblock pages
/// and the arena to `arena_used` (nothing of the padding), a read transaction reads nothing more,
/// and only the first commit reads the device again (whole, for the writer's padded `Store`).
#[test]
fn open_reads_the_live_image_once_and_the_writer_only_at_the_first_write() {
    // a 64 KiB image with four small commits, PADDED to its capacity on the device (`Store::to_bytes`,
    // what a restore or a copy of a whole image holds): arena_used is a few hundred cells of 8192
    let db = Db::create(Detached, Options::detached().initial_bytes(64 * 1024)).unwrap();
    for i in 0..4u8 {
        let mut w = db.begin_write().unwrap();
        w.put(format!("k{i}").as_bytes(), b"v").unwrap();
        w.commit(Durability::None).unwrap();
    }
    let padded = Store::from_bytes(&db.snapshot_bytes()).cells;
    let disk = crate::sim::SimDisk::new(padded);
    let total = disk.len_cells().unwrap();
    let used = crate::format::pick_checked_in(&View::new(&db_bytes(&disk))).unwrap().arena_used as usize;
    assert!(used + 512 < total, "the fixture must be padded past its arena: used {used} of {total}");

    let (cells, calls) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    let counts = || (cells.load(Ordering::SeqCst), calls.load(Ordering::SeqCst));
    let dev = Counted { inner: disk, cells: Arc::clone(&cells), calls: Arc::clone(&calls) };
    let db = Db::open(dev, Options::default()).unwrap();
    let (c_open, n_open) = counts();
    assert_eq!(c_open, used, "open reads exactly the live image: {c_open} cells, arena_used {used}");
    assert_eq!(n_open, 2, "the superblock pages, then the arena");

    let r = db.begin_read();
    assert_eq!(r.get(b"k3").unwrap().unwrap(), &b"v"[..]);
    assert_eq!(r.len(), 4);
    assert!(db.verify(Verify::WholeArena).is_ok());
    drop(r);
    assert_eq!(counts(), (c_open, n_open), "reads and verify read nothing more from the device");

    let mut w = db.begin_write().unwrap();
    w.put(b"k4", b"v").unwrap();
    assert_eq!(counts(), (c_open, n_open), "a transaction takes no writer (P19b): only its commit does");
    w.commit(Durability::Data).unwrap();
    assert_eq!(db.begin_read().len(), 5);
    assert_eq!(counts(), (c_open + total, n_open + 1), "the first commit reads the device whole, once");
}

fn db_bytes<D: Device>(disk: &D) -> Vec<u8> {
    crate::cells_to_bytes(&disk.read_cells(0, disk.len_cells().unwrap()).unwrap())
}

#[test]
fn the_last_commit_rule_names_the_object_or_the_anchor_and_never_falls_back_over_bytes() {
    let clean = v3_bytes();
    let mark = mark_of(&clean);
    let v = View::new(&clean);
    assert!(bebop_image::obj_cells_in(&v, mark) > 0);
    let k = crate::format::pick_checked_in(&v).unwrap().generation;

    // (a) an object of the last commit changed, its crc not re-sealed: BAD_CRC naming it
    let mut bad = clean.clone();
    bad[(mark + 2 + bebop_image::obj_cells_in(&v, mark) - 1) * 8] ^= 1; // its last payload cell: a value byte
    assert!(matches!(crate::format::anchor_in(&View::new(&bad), &crate::format::pick_checked_in(&View::new(&bad)).unwrap()), crate::format::Anchor::Torn { .. }));
    assert!(matches!(Db::from_bytes(&bad, Options::detached()), Err(Refusal::BadCrc(b)) if b.obj == mark));

    // (b) the same object re-sealed over its changed byte: every object checks, the anchor is stale
    let mut st = Store::from_bytes(&bad);
    st.seal(mark);
    let stale = st.to_bytes_trimmed();
    let e = Db::from_bytes(&stale, Options::detached()).err().unwrap();
    assert!(matches!(e, Refusal::LastCommit(crate::sim::Why::Anchor(crate::format::Anchor::Torn { .. }))), "{e:?}");
    // the same refusal at every depth: the rule runs before the KV is read
    for depth in [Verify::Reachable, Verify::WholeArena] {
        assert!(matches!(Db::from_bytes(&stale, Options::detached().verify(depth)), Err(Refusal::LastCommit(_))));
    }
    // (c) a reader with RECOVERY AUTHORITY (a file's rule) opens generation k-1 instead
    let fell = Db::from_bytes(&stale, Options::detached().on_torn(OnTorn::Fallback)).unwrap();
    assert_eq!(fell.begin_read().generation().get(), k - 1);
    assert_eq!(fell.begin_read().get(b"k07").unwrap().unwrap(), &b"base 7"[..], "generation k-1 has the base's k07, not the last commit's");

    // POSITIVE TWIN: re-anchored over the changed bytes (what a writer does), it opens as generation k.
    let mut st = Store::from_bytes(&stale);
    let sb = st.pick().unwrap();
    let used = st.cells[sb.at + crate::format::SB_USED];
    st.cells[sb.at + crate::format::SB_ANC] = crate::format::anchor_of(&st, mark as i64, used);
    st.cells[sb.at + crate::format::SB_CRC] = bebop_image::crc32_cells(&st.cells, sb.at, 15) as i64;
    let db = Db::from_bytes(&st.to_bytes_trimmed(), Options::detached()).unwrap();
    assert_eq!(db.begin_read().generation().get(), k);
    assert_ne!(db.begin_read().get(b"k07").unwrap().unwrap(), &b"base 7"[..], "generation k has the last commit's k07 (one byte of it flipped)");
}
