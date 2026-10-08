//! W-AE over the object itself (MemHost): the point a flush writes, the point every NTH write
//! writes, the budget cap, and the absent binding that must not fail anything. Three are the
//! row's RED proofs: `a_flush_writes_one_point...` (writer not called at flush), `the_doubles...`
//! (wrong order), `a_missing_binding...` (an absent binding failing the request).

use super::super::super::host::mem::{Harness, T0};
use super::{Point, CAP_PER_DAY, DOUBLES, NTH};
use serde_json::Value;

fn points(h: &Harness) -> Vec<Point> {
    h.obj.counters.fake_ae.borrow().clone().expect("the fake binding is present")
}

fn flush(h: &Harness) -> Value {
    let r = h.get("/fold/counters?flush=1");
    assert_eq!(r.status_code(), 200);
    r.body_value()
}

fn writes(h: &Harness, from_gen: i64, n: i64) {
    for g in from_gen..from_gen + n {
        assert_eq!(h.put("settings", g, format!("s{g}").as_bytes()).status_code(), 200);
    }
}

/// One order placed (an append to the log image), as AX0's tests do.
fn append(h: &Harness, generation: i64, id: &str) {
    let placed = serde_json::json!({ "kind": dowiz_hub::EventKind::Placed as u8, "order_id": id,
        "payload": serde_json::json!({ "id": id, "status": "PENDING", "total": 1200 }).to_string(), "clock": T0 as u64 });
    let c = crate::wire::Call::new("https://hub/fold/append", worker::Method::Post).unwrap()
        .with_header("x-generation", &generation.to_string()).with_json(&placed);
    assert_eq!(h.call(c).status_code(), 200);
}

fn d(p: &Point, name: &str) -> f64 {
    p.doubles[DOUBLES.iter().position(|n| *n == name).unwrap()]
}

/// RED-able: the nightly's flush writes ONE point for the window, with the venue as index and blob1,
/// the cause and the clock as blobs 2 and 3 -- and a second flush with nothing new writes nothing.
#[test]
fn a_flush_writes_one_point_for_the_window() {
    let h = Harness::new();
    writes(&h, 0, 2);
    h.get("/fold/changes?since=0");
    assert!(points(&h).is_empty(), "nothing is written before a flush or the NTH write");
    flush(&h);
    let p = points(&h);
    assert_eq!(p.len(), 1, "{p:?}");
    assert_eq!(p[0].index, "v1");
    assert_eq!(p[0].blobs, ["v1".to_string(), "flush".into(), "native-us".into()]);
    assert_eq!(p[0].doubles.len(), DOUBLES.len());
    // The counters GET that flushed is a read, so the point after it holds exactly that read.
    flush(&h);
    let p = points(&h);
    assert_eq!(p.len(), 2, "{p:?}");
    assert_eq!((d(&p[1], "reads"), d(&p[1], "writes"), d(&p[1], "wakes_total")), (1.0, 0.0, 0.0), "{p:?}");
}

/// RED-able: the doubles are in `DOUBLES` order, which is the collector's contract (double1..15).
#[test]
fn the_doubles_are_in_contract_order() {
    let h = Harness::new();
    for (g, id) in [(0, "o1"), (1, "o2"), (2, "o3")] {
        append(&h, g, id);
    }
    h.get("/fold/changes?since=1"); // answered: since_total 1, since_none 0
    h.get("/fold/changes?since=-9"); // not answerable: since_none 1
    flush(&h);
    let v = &points(&h)[0].doubles;
    // since_total, since_none, wakes_total, wakes_writing, reads, writes
    assert_eq!(&v[..6], &[2.0, 1.0, 1.0, 1.0, 3.0, 3.0], "{v:?}");
    assert!(v[6] >= 3.0, "proj_rows: at least one chunk per write: {v:?}");
    assert_eq!(&v[10..], &[0.0; 5], "no catalogue write: {v:?}");
    assert_eq!(DOUBLES[0], "since_total");
    assert_eq!(DOUBLES[14], "journal_bytes");
}

/// RED-able: AN ABSENT BINDING FAILS NOTHING. Writes and the flush still answer 200, the error is
/// counted and said in `ae`, and the counts are KEPT: once the binding is back, the next point
/// carries them.
#[test]
fn a_missing_binding_does_not_fail_the_request_and_loses_nothing() {
    let h = Harness::new();
    *h.obj.counters.fake_ae.borrow_mut() = None;
    writes(&h, 0, NTH as i64); // the NTH write tries, is refused, and still answers 200
    let f = flush(&h);
    assert!(f["ae"]["errors"].as_u64().is_some_and(|n| n >= 2), "NTH + flush both refused: {f}");
    assert!(f["ae"]["lastError"].as_str().is_some_and(|e| e.contains("COUNTERS")), "{f}");
    assert_eq!(f["ae"]["points"], 0, "{f}");
    *h.obj.counters.fake_ae.borrow_mut() = Some(Vec::new());
    flush(&h);
    let p = points(&h);
    assert_eq!(p.len(), 1, "{p:?}");
    assert_eq!((d(&p[0], "writes"), d(&p[0], "wakes_total")), (NTH as f64, 1.0), "the kept delta: {p:?}");
}

/// Every NTH stored write writes a point; fewer do not.
#[test]
fn every_nth_write_writes_a_point() {
    let h = Harness::new();
    writes(&h, 0, NTH as i64 - 1);
    assert!(points(&h).is_empty());
    writes(&h, NTH as i64 - 1, 1);
    let p = points(&h);
    assert_eq!(p.len(), 1, "{p:?}");
    assert_eq!((p[0].blobs[1].as_str(), d(&p[0], "writes"), d(&p[0], "wakes_writing")), ("nth", NTH as f64, 1.0));
    writes(&h, NTH as i64, NTH as i64);
    let p = points(&h);
    assert_eq!((p.len(), d(&p[1], "writes"), d(&p[1], "wakes_total"), d(&p[1], "wakes_writing")), (2, NTH as f64, 0.0, 0.0), "deltas: {p:?}");
}

/// THE BUDGET: one object writes at most CAP_PER_DAY NTH-points a day; past it the counts wait
/// for the flush, which is not capped. 1,000 hubs x (CAP + 1 flush) stays under half of the Free
/// plan's 100,000 points/day.
#[test]
fn the_daily_cap_holds_and_the_flush_still_carries_the_rest() {
    assert!(1000 * (u64::from(CAP_PER_DAY) + 1) <= 100_000 / 2);
    let h = Harness::new();
    let n = NTH as i64 * (i64::from(CAP_PER_DAY) + 2);
    writes(&h, 0, n);
    assert_eq!(points(&h).len(), CAP_PER_DAY as usize, "capped");
    flush(&h);
    let p = points(&h);
    assert_eq!(p.len(), CAP_PER_DAY as usize + 1);
    let total: f64 = p.iter().map(|x| d(x, "writes")).sum();
    assert_eq!(total, n as f64, "every write counted exactly once across the points");
}

/// A cold object (an eviction) starts a new delta with its own wake; a fold from bytes is sampled.
#[test]
fn a_woken_object_counts_its_wake_and_its_cold_fold() {
    let h = Harness::new();
    append(&h, 0, "o1");
    let cold = h.cold();
    cold.get("/fold/orders");
    flush(&cold);
    let p = points(&cold);
    assert_eq!((d(&p[0], "wakes_total"), d(&p[0], "wakes_writing"), d(&p[0], "writes")), (1.0, 0.0, 0.0), "{p:?}");
    assert_eq!(d(&p[0], "cold_folds"), 1.0, "{p:?}");
    assert_eq!(d(&p[0], "cold_fold_p50_us"), d(&p[0], "cold_fold_us"), "one fold: its p50 is itself");
}
