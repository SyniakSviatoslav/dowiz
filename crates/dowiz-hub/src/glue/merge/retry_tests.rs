//! W-MG-TIDY: RETRIES THE `glued` NOTE'S CHAIN CANNOT SEE. `a_longer_retry_...` (tests/atk4_s2.rs)
//! holds because a longer tail's chain passes through the id the note names. A SHORTER tail -- the
//! older request delivered after the newer one, a tablet restored from a stale copy -- stops
//! below that id, and a fork of the tablet leaves it, so neither finds the note.

use super::super::tablet;
use super::*;
use crate::stock::{StockEvent, StockLog};

const WASTE: &str = r#"{"k":"wasted","item":"salmon","qty":1,"reason":"spoiled","by":"ana"}"#;

fn hub(sha: bool) -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.set_entropy([5; 32]);
    s.set_sha256(sha);
    s.set_clock(1000);
    s.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    s
}

fn left(h: &StockLog, item: &str) -> i64 {
    h.ledger().unwrap().items().into_iter().find(|(i, _)| i == item).map_or(0, |(_, l)| l.on_hand)
}

fn go(h: &mut StockLog, t: &Tail) -> Reply {
    let bytes = h.to_bytes_trimmed();
    merge(&bytes, h, t).unwrap()
}

/// The two tails of one tablet: one record, then two, both from `base`.
fn two_tails(h: &StockLog, ctx: &str) -> (Tail, Tail, Vec<u8>) {
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, ctx, WASTE, 2000).unwrap();
    let (b, _) = tablet::record(&a, ctx, WASTE, 3000).unwrap();
    (tablet::tail(&a, ctx, &base).unwrap(), tablet::tail(&b, ctx, &base).unwrap(), b)
}

/// The older, shorter request lands after the longer one.
#[test]
fn a_shorter_retry_adopts_nothing() {
    for sha in [true, false] {
        let mut h = hub(sha);
        let base = h.tip_id();
        let (short, long, b) = two_tails(&h, "kitchen");
        assert_eq!(go(&mut h, &long).merged, 2, "control");
        let tip = h.tip_id();
        let r = go(&mut h, &short);
        assert_eq!(left(&h, "salmon"), 98, "HOLE: 2 movements took {} from the shelf (sha={sha}): {r:?}", 100 - left(&h, "salmon"));
        assert!(r.replay && r.merged == 0 && r.law(1), "{r:?}");
        assert_eq!(h.tip_id(), tip, "a replay writes nothing");
        // and the full tablet, which never saw either reply, still lands on the hub
        let r = go(&mut h, &long);
        let reb = tablet::rebase(&b, &base, false, 2, &r.entries, &r.tip).unwrap();
        assert_eq!(StockLog::load(&reb).unwrap().tip_id(), h.tip_id());
    }
}

/// A bare tablet record on a flipped hub is adopted with a nonce put in front, so its bytes are
/// not the bytes sent; the body is.
#[test]
fn a_shorter_retry_of_a_record_the_hub_gave_a_nonce() {
    let mut h = hub(false);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let bare = |at: &[u8], when: i64| {
        let mut l = StockLog::load(at).unwrap();
        l.set_sha256(false);
        l.set_clock(when);
        l.append(&StockEvent::Received { item: "salmon".into(), qty: 2 }).unwrap();
        l.to_bytes_trimmed()
    };
    let a = bare(&img, 2000);
    let b = bare(&a, 3000);
    let short = tablet::tail(&a, "kitchen", &base).unwrap();
    let long = tablet::tail(&b, "kitchen", &base).unwrap();
    h.set_sha256(true);
    assert_eq!(go(&mut h, &long).merged, 2, "control");
    let r = go(&mut h, &short);
    assert_eq!(left(&h, "salmon"), 104, "{r:?}");
}

/// A fork: the tablet came back from a copy holding the first record and wrote another. The first
/// is known, the new one is new.
#[test]
fn a_forked_retry_adopts_only_what_is_new() {
    let mut h = hub(true);
    let (_, long, _) = two_tails(&h, "kitchen");
    let base = h.tip_id();
    let img = h.to_bytes_trimmed();
    go(&mut h, &long);
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let (c, _) = tablet::record(&a, "kitchen", WASTE, 4000).unwrap();
    let r = go(&mut h, &tablet::tail(&c, "kitchen", &base).unwrap());
    assert_eq!((r.skipped, r.merged), (1, 1), "{r:?}");
    assert_eq!(left(&h, "salmon"), 97, "three movements, three off the shelf");
}

/// A record the ledger HELD is known too: its retry is no second decision and no late adoption.
#[test]
fn a_shorter_retry_of_a_held_record_is_no_second_decision() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", r#"{"k":"reserved","item":"salmon","qty":60,"order":"o1"}"#, 2000).unwrap();
    let (b, _) = tablet::record(&a, "kitchen", WASTE, 3000).unwrap();
    h.append(&StockEvent::Wasted { item: "salmon".into(), qty: 90, reason: crate::stock::WasteReason::Spoiled, by: "ana".into() }).unwrap();
    let r = go(&mut h, &tablet::tail(&b, "kitchen", &base).unwrap());
    assert!(r.held.iter().any(|(p, _)| *p == 0), "control: {r:?}");
    let open = crate::glue::desk::open(&h).len();
    h.append(&StockEvent::Received { item: "salmon".into(), qty: 1000 }).unwrap();
    let r = go(&mut h, &tablet::tail(&a, "kitchen", &base).unwrap());
    assert!(r.replay && r.held.is_empty() && r.merged == 0, "{r:?}");
    assert_eq!(crate::glue::desk::open(&h).len(), open, "no second decision");
}

/// Twins: the rule does not swallow a real movement.
#[test]
fn another_context_with_the_same_movement_is_not_a_retry() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (_, long, _) = two_tails(&h, "kitchen-a1");
    go(&mut h, &long);
    let (a, _) = tablet::record(&img, "kitchen-b2", WASTE, 2000).unwrap();
    let r = go(&mut h, &tablet::tail(&a, "kitchen-b2", &base).unwrap());
    assert_eq!((r.skipped, r.merged), (0, 1), "{r:?}");
    assert_eq!(left(&h, "salmon"), 97);
}

/// Twin: one context, one ms, one body, but another `prev`: another nonce, so a second movement.
#[test]
fn a_second_record_with_the_same_body_is_a_movement() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    h.append(&StockEvent::Received { item: "salmon".into(), qty: 1 }).unwrap();
    let (later, img2) = (h.tip_id(), h.to_bytes_trimmed());
    go(&mut h, &tablet::tail(&a, "kitchen", &base).unwrap());
    let (b, _) = tablet::record(&img2, "kitchen", WASTE, 2000).unwrap();
    let r = go(&mut h, &tablet::tail(&b, "kitchen", &later).unwrap());
    assert_eq!((r.skipped, r.merged), (0, 1), "{r:?}");
}

/// PINNED, NOT WANTED (W-MG-TIDY). Two devices on ONE context, one image, one movement in one ms:
/// a tablet has no seed, the derived nonce is the same, so the record and its id are the same and
/// the hub cannot tell them from a retry (H10's contract: one context, one tablet). `tablet.js`
/// keeps a pre-existing image's `kitchen` context, so the case is reachable. It takes device
/// entropy in `tablet::record`, across the wasm surface.
#[test]
fn two_devices_on_one_context_in_one_ms_read_as_one_movement() {
    let mut h = hub(true);
    let (base, img) = (h.tip_id(), h.to_bytes_trimmed());
    let (a, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    let (b, _) = tablet::record(&img, "kitchen", WASTE, 2000).unwrap();
    assert_eq!(a, b, "control: byte for byte one image");
    go(&mut h, &tablet::tail(&a, "kitchen", &base).unwrap());
    let r = go(&mut h, &tablet::tail(&b, "kitchen", &base).unwrap());
    assert!(r.replay, "{r:?}");
    assert_eq!(left(&h, "salmon"), 99, "the second device's movement is not on the shelf");
}
