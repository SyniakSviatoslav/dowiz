//! T-ES-51 (AC-ES-51): a point read through the mmapped image allocates nothing beyond the value.
//! P3's `ReadTxn::get` over `MmapRoDevice` does not exist yet; today's equivalent is
//! `KvIn::get` over a `View` of the mmapped file, which is what the bench's bebop adapter reads
//! through. A counting `GlobalAlloc` measures the bytes allocated INSIDE the get loop only (the
//! read transaction's own setup -- `KvIn::reopen` -- is outside it and stated, not hidden).
//! ONE test in this binary: the counter is process-wide, so a second test thread would pollute it.
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use bebop_store::kv::zc::KvIn;
use bebop_store::View;
use es_bench::bebop::Bebop;
use es_bench::data::{entries, key, val, Mmap};
use es_bench::engine::{Engine, W};

struct Counting;
static BYTES: AtomicU64 = AtomicU64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        BYTES.fetch_add(l.size() as u64, Relaxed);
        CALLS.fetch_add(1, Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        BYTES.fetch_add(n as u64, Relaxed);
        CALLS.fetch_add(1, Relaxed);
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Counting = Counting;

/// (allocation calls, bytes) over `gets` of the keys `ks` on the image in `dir`.
fn count_gets(dir: &std::path::Path, ks: &[Vec<u8>]) -> (u64, u64, u64) {
    let m = Mmap::open(&dir.join("kv.bebop"));
    let kv = KvIn::open(View::new(m.bytes())).unwrap();
    let mut vbytes = 0u64;
    let (c0, b0) = (CALLS.load(Relaxed), BYTES.load(Relaxed));
    for k in ks {
        vbytes += kv.get(k).unwrap().expect("present").len() as u64;
    }
    (CALLS.load(Relaxed) - c0, BYTES.load(Relaxed) - b0, vbytes)
}

#[test]
fn t_es_51_get_allocates_nothing_beyond_the_value() {
    let dir = std::env::temp_dir().join(format!("es-bench-allocs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let e = entries(2_000, 256);
    let mut db = Bebop::load(&dir, &e);
    let ks: Vec<Vec<u8>> = (0..2_000u64).step_by(7).map(key).collect();

    // A compacted image (no delta chain): every value is a slice of the map.
    let (calls, bytes, _) = count_gets(&dir, &ks);
    assert_eq!((calls, bytes), (0, 0), "{} gets on a compacted image allocated {calls} times / {bytes} B", ks.len());

    // With a delta chain on the file: a value read from a delta record may be copied out (its
    // bytes are packed into cells), but never more than the value itself.
    let k1 = key(3);
    let v1 = val(3, 256, 9);
    db.commit(&[W::Put(&k1, &v1)]);
    assert_eq!(db.deltas, 1, "the put must land as a delta, not a compaction: {}", es_bench::engine::Engine::note(&db));
    let mut ks2 = ks.clone();
    ks2.push(k1.clone());
    let (calls, bytes, vbytes) = count_gets(&dir, &ks2);
    assert!(bytes <= vbytes, "{} gets over a delta chain allocated {bytes} B in {calls} calls, values total {vbytes} B", ks2.len());
    println!("T-ES-51: compacted 0 allocations; with a chain {calls} allocations / {bytes} B for {vbytes} B of values");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
