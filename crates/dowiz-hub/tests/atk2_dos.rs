//! W-MG-ATK2: A READER OPENS FILES FROM STRANGERS. `chain_crc_at` and `read_at` carry a budget
//! because records can overlap (W-AUDIT S2: 9 GB from a 1 MB log); nothing else on `verify`'s path
//! does. Each shape here needs no key and no valid signature. RED ON PURPOSE until bounded.
//! Sizes are small so a red run ends; `ATK2_FULL=1` prints the 1 MB numbers.
#[path = "atk2_dos/shapes.rs"]
mod shapes;

use einstore::evlog::doc::seg::SEG_VERSION;
use einstore::evlog::doc::{self, check_anchors, Anchor, AnchorCheck, Doc, Trust, Verdict};
use einstore::Store;
use einstore::evlog::doc::Signer as _;
use dowiz_hub::doc_seal::{DeviceKey, HubIds, MlDsa65, SealedHub};
use dowiz_hub::EventKind;
use shapes::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Mutex;
use std::time::Instant;

struct Peak;
static NOW: AtomicUsize = AtomicUsize::new(0);
static TOP: AtomicUsize = AtomicUsize::new(0);
static ONE: Mutex<()> = Mutex::new(());
unsafe impl GlobalAlloc for Peak {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let n = NOW.fetch_add(l.size(), SeqCst) + l.size();
        TOP.fetch_max(n, SeqCst);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        NOW.fetch_sub(l.size(), SeqCst);
        System.dealloc(p, l)
    }
}
#[global_allocator]
static P: Peak = Peak;

/// Seconds and peak heap above the file of `f`, the answer printed.
fn measure(name: &str, bytes: &[u8], f: &dyn Fn(&Store) -> String) -> (f64, usize) {
    let st = Store::from_bytes(bytes);
    let at = NOW.load(SeqCst);
    TOP.store(at, SeqCst);
    let t = Instant::now();
    let said = f(&st);
    let (s, peak) = (t.elapsed().as_secs_f64(), TOP.load(SeqCst) - at);
    println!("ATK2 {name}: file={} B time={s:.3}s peak_heap={peak} B ({}x file) {said}", bytes.len(), peak / bytes.len());
    (s, peak)
}
fn verify(st: &Store) -> String {
    match doc::verify(st, &HubIds, &MlDsa65, Trust::Genesis) {
        Ok(r) => format!("seals_bad={} records={}", r.counters.seals_bad, r.counters.records),
        Err(e) => format!("Err {e:?}").chars().take(60).collect(),
    }
}
struct Yes;
impl AnchorCheck for Yes {
    fn check(&self, _: &Anchor) -> Verdict {
        Verdict::Unchecked(String::new())
    }
}
fn anchors(st: &Store) -> String {
    match check_anchors(st, &Yes) {
        Ok(c) => format!("anchors={}", c.len()),
        Err(e) => format!("Err {e:?}").chars().take(60).collect(),
    }
}

/// Bounded means: the heap of one verify stays within `K`x the file, plus a fixed 1 MB.
const K: usize = 8;
fn bounded(name: &str, bytes: usize, peak: usize) {
    assert!(peak <= bytes * K + (1 << 20), "HOLE {name}: {bytes} B file took {peak} B of heap");
}

#[test]
fn the_dearest_file_read_stays_within_k() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let f = many_actors(1000);
    let (_, p) = measure("actors-1000", &f, &verify);
    bounded("actors", f.len(), p);
}

#[test]
fn nested_segments_are_refused_before_the_allocation() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let (a, b) = (nested_segs(8 << 10), nested_segs(16 << 10));
    let (_, p1) = measure("segs-64K", &a, &verify);
    let (_, p2) = measure("segs-128K", &b, &verify);
    println!("ATK2 segs growth: x{:.2} heap for x2 file", p2 as f64 / p1 as f64);
    bounded("segs", b.len(), p2);
}

#[test]
fn nested_anchors_are_refused_before_the_allocation() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let (a, b) = (nested_anchors(8 << 10), nested_anchors(16 << 10));
    let (_, p1) = measure("anch-64K", &a, &anchors);
    let (_, p2) = measure("anch-128K", &b, &anchors);
    println!("ATK2 anch growth: x{:.2} heap for x2 file", p2 as f64 / p1 as f64);
    bounded("anchors", b.len(), p2);
}

#[test]
fn one_key_on_many_rows_is_paid_for_once() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let f = one_key_many_rows(2000);
    let (_, p) = measure("keys-2000-rows", &f, &verify);
    bounded("keys", f.len(), p);
}

/// Double heads, records and seal size: a bounded verify is about 2x. A ratio and not a stopwatch,
/// so a slow box does not decide; under 1 s is under the clock's noise here.
#[test]
fn a_ring_of_two_seals_is_walked_once() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let (a, b) = (seal_ring(100, 200, 256), seal_ring(200, 400, 512));
    let (t1, _) = measure("ring-1", &a, &verify);
    let (t2, p) = measure("ring-2", &b, &verify);
    println!("ATK2 ring growth: x{:.2} time for x{:.2} file", t2 / t1, b.len() as f64 / a.len() as f64);
    assert!(t2 < 1.0 || t2 / t1 < 4.0, "HOLE ring: x{:.2} time", t2 / t1);
    let r = doc::verify(&Store::from_bytes(&b), &HubIds, &MlDsa65, Trust::Genesis).unwrap();
    assert_eq!(r.counters.seals_ok, 0, "two seals of nobody");
    assert!(r.counters.seals_bad >= 3, "the ring itself is counted: {:?}", r.counters);
    bounded("ring", b.len(), p);
}

/// The writers' unchecked read stops at the budget and keeps what it paid for.
#[test]
fn the_plain_anchor_read_is_bounded_too() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let f = nested_anchors(16 << 10);
    let (_, p) = measure("read-anchors", &f, &|st| {
        let v = Doc::open(st).unwrap();
        format!("anchors={}", einstore::evlog::doc::anch::read_anchors(st, v.anch).len())
    });
    bounded("read_anchors", f.len(), p);
}

/// A row count of 2^62: `m * 10` panics in a debug build and wraps on wasm32. Not a SEG, by name.
#[test]
fn a_segment_claiming_two_to_the_sixty_two_rows_is_no_segment() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let mut st = base(1024, 1);
    let mut tx = st.begin().unwrap();
    let fill = |st: &mut Store, p: usize, _: usize| {
        st.cells[p + 2] = SEG_VERSION;
        st.cells[p + 2 + 11] = 1 << 62;
    };
    let s = nest(&mut st, &mut tx, 256, einstore::evlog::doc::seg::DIGEST_SEG, 12, 10, 7, &fill);
    let f = commit(&mut st, &mut tx, Refs { seg: Some(s), ..Refs::default() });
    assert!(matches!(doc::verify(&Store::from_bytes(&f), &HubIds, &MlDsa65, Trust::Genesis), Err(doc::DocError::Corrupt(_))));
}

/// The walks share ONE budget: rows that fit the image alone are refused on top of the records.
#[test]
fn records_and_keys_pay_from_one_budget() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let f = records_then_keys(9000, 20);
    let got = doc::verify(&Store::from_bytes(&f), &HubIds, &MlDsa65, Trust::Genesis);
    assert!(matches!(got, Err(doc::DocError::Corrupt(_))), "the sum is over the image: {} B, {:?}", f.len(), got.map(|r| r.counters));
    let twin = records_then_keys(9000, 1);
    assert!(doc::verify(&Store::from_bytes(&twin), &HubIds, &MlDsa65, Trust::Genesis).is_ok(), "the twin fits");
}

#[test]
fn nested_seals_are_refused_before_the_allocation() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let f = nested_seals(32 << 10);
    let (_, p) = measure("seals-256K", &f, &verify);
    bounded("seals", f.len(), p);
    assert!(matches!(doc::verify(&Store::from_bytes(&f), &HubIds, &MlDsa65, Trust::Genesis), Err(doc::DocError::Corrupt(_))));
}

/// A HEADS row is signed by nobody: one that disagrees with a seal another row reached still
/// costs that seal, whichever row was read first.
#[test]
fn a_second_row_that_disagrees_costs_the_seal() {
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let a = DeviceKey::from_seed(&[0xA1; 32]);
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    sh.append(EventKind::Placed, "o1", "{}", &a, 1).unwrap();
    let f = sh.to_bytes_trimmed();
    let roots = [a.id()];
    assert!(doc::verify(&Store::from_bytes(&f), &HubIds, &MlDsa65, Trust::Only(&roots)).unwrap().holds(), "the twin");
    let r = doc::verify(&Store::from_bytes(&second_row(&f)), &HubIds, &MlDsa65, Trust::Only(&roots)).unwrap();
    assert_eq!((r.counters.seals_ok, r.counters.seals_bad), (0, 1), "{:?}", r.counters);
}

/// The full-size numbers: `ATK2_FULL=1`, the test BINARY under `ulimit -v`, never cargo.
#[test]
fn full_size_files() {
    if std::env::var("ATK2_FULL").is_err() {
        return;
    }
    let _o = ONE.lock().unwrap_or_else(|e| e.into_inner());
    let shapes: [(&str, Vec<u8>, &dyn Fn(&Store) -> String); 5] = [
        ("keys-1M", one_key_many_rows(24000), &verify),
        ("ring-1M", seal_ring(5000, 4000, 8192), &verify),
        ("segs-1M", nested_segs(130 << 10), &verify),
        ("anch-1M", nested_anchors(131_000), &anchors),
        ("actors-1M", many_actors(7000), &verify),
    ];
    for (name, f, run) in &shapes {
        let (t, p) = measure(name, f, *run);
        bounded(name, f.len(), p);
        assert!(t < 60.0, "{name}: {t:.1}s");
    }
}
