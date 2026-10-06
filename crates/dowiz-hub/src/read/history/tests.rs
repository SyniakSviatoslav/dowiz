//! `history()` READS ONE ORDER'S RECORDS AND ANSWERS WHAT THE WHOLE WALK ANSWERED.
//! The reference is the old definition, verbatim: `events()` filtered by order id and
//! `is_order()`, oldest first. Seeded logs, one named corrupted cell each; no fuzzing.
use std::collections::HashMap;

use bebop_store::Store;
use sha2::{Digest, Sha256};

use crate::{Event, EventKind, Hub};

/// The old `history()` for EVERY id at once: one `events()` pass, grouped. Grouping keeps
/// each id's events in walk order, so this is `events().filter(id).reverse()` per id.
fn walked(h: &Hub) -> HashMap<String, Vec<Event>> {
    let mut by: HashMap<String, Vec<Event>> = HashMap::new();
    for e in h.events_oldest_first() {
        if e.kind.is_order() {
            by.entry(e.order_id.clone()).or_default().push(e);
        }
    }
    by
}

/// Every id the log names (orders AND audit subjects), plus ids it never saw.
fn ids(h: &Hub) -> Vec<String> {
    let mut v: Vec<String> = h.events().into_iter().map(|e| e.order_id).collect();
    v.extend(["ord_", "ord_9999999", "", "cust:", "o"].map(String::from));
    v.sort();
    v.dedup();
    v
}

fn assert_equal(h: &Hub, what: &str) -> usize {
    let want = walked(h);
    let mut checked = 0;
    for id in ids(h) {
        let got = h.history_scan(&id);
        assert_eq!(&got, want.get(&id).unwrap_or(&Vec::new()), "{what}: history({id:?})");
        assert_eq!(h.history(&id), got, "{what}: history() is the scan");
        checked += 1;
    }
    checked
}

const KINDS: [EventKind; 4] = [EventKind::Placed, EventKind::Advanced, EventKind::Paid, EventKind::Amended];

/// `orders` orders, `per` events each on average, interleaved by a seeded LCG. Ids share
/// prefixes (`ord_1`, `ord_10`, `ord_100`); one is 200 bytes; one is empty; every fifth
/// event names an actor (a v2 record's payload then starts four cells later); every
/// seventh is an audit `Revealed` under a `cust:` subject. Born at 64 KiB, so it grows.
fn venue(orders: u64, events: u64, seed: u64) -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let mut x = seed;
    let long = format!("ord_{}", "L".repeat(196));
    for i in 0..events {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let k = (x >> 33) % orders;
        let id = match k {
            0 => long.clone(),
            1 => String::new(),
            _ => format!("ord_{k}"),
        };
        let actor = if i % 5 == 0 { [0x7E; 32] } else { [0u8; 32] };
        let json = format!(r#"{{"id":"{id}","n":{i},"pad":"{}"}}"#, "p".repeat((x >> 20) as usize % 23));
        if i % 7 == 3 {
            h.append(EventKind::Revealed, &format!("cust:{k}"), r#"{"by":"a"}"#, i, actor).unwrap();
        } else {
            h.append(KINDS[(x >> 40) as usize % 4], &id, &json, i, actor).unwrap();
        }
    }
    h
}

/// THE NAMED EQUALITY TEST: >= 1000 orders, ~3000 events, every id the log names plus five
/// it does not -- on the live hub and on the same image reloaded from its bytes.
#[test]
fn scan_equals_the_walk_on_1000_orders() {
    let h = venue(1100, 3300, 0x0C4A_2026_1006);
    assert!(h.to_bytes().len() > 64 * 1024, "the log grew on the way");
    let n = assert_equal(&h, "live");
    assert!(n > 1000, "checked {n} ids");
    let back = Hub::load(&h.to_bytes_trimmed()).unwrap();
    assert_equal(&back, "reloaded");
    let some = back.history("ord_10");
    assert!(!some.is_empty() && some.iter().all(|e| e.order_id == "ord_10"), "ord_1 is not ord_10");
}

/// The `at`-th record of a newest-first walk.
fn nth(st: &Store, at: usize) -> usize {
    let mut o = st.follow(st.root().unwrap(), 1).unwrap();
    for _ in 0..at {
        o = st.follow(o, 2).unwrap();
    }
    o
}

/// A QUARANTINED RECORD IS NEVER HISTORY. One changed JSON byte in a record of `ord_q`
/// (crc fails, prefix intact, so the scan DOES find it) and one changed id byte in another
/// (the prefix no longer matches): neither comes back, the other two do, and the answer
/// still equals the walk's.
#[test]
fn a_quarantined_record_is_never_returned_as_history() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    for (i, id) in ["ord_q", "ord_x", "ord_q", "ord_q", "ord_q"].iter().enumerate() {
        h.append(EventKind::Placed, id, &format!(r#"{{"step":{i},"note":"abcdefghij"}}"#), i as u64, [0u8; 32]).unwrap();
    }
    let mut st = Store::from_bytes(&h.to_bytes());
    // newest first: at 0 = step 4, at 1 = step 3, at 2 = step 2, at 3 = step 1 (`ord_x`), at 4 = step 0
    let json_cell = nth(&st, 0);
    st.cells[json_cell + 2 + 12 + 2] ^= 0x100; // payload cell 2: JSON bytes, not the id
    let id_cell = nth(&st, 2);
    st.cells[id_cell + 2 + 12] ^= 0x1_0000; // payload byte 2: the id's first byte
    let bad = Hub::load(&st.to_bytes()).expect("quarantined, not refused");
    assert_eq!(bad.quarantined().len(), 2);
    let got = bad.history("ord_q");
    let steps: Vec<bool> = (0..5).map(|s| got.iter().any(|e| e.order_json.contains(&format!(r#""step":{s}"#)))).collect();
    assert_eq!(steps, vec![true, false, false, true, false], "steps 0 and 3 only: {got:?}");
    assert_equal(&bad, "two quarantined");
}

/// NOTHING TO GO STALE: after a GROW, a ROTATION and a REDACTION REBUILD -- each of which
/// replaces the store under the hub -- and appends after each, the answer is still the
/// walk's. (There is no index; this test is what would catch one being added.)
#[test]
fn after_grow_rotate_and_redact_history_still_equals_the_walk() {
    let mut h = venue(60, 200, 7);
    let before = h.to_bytes().len();
    for i in 0..400u64 {
        h.append(EventKind::Advanced, &format!("ord_{}", i % 50), r#"{"_d":true,"status":"READY"}"#, 1000 + i, [0u8; 32]).unwrap();
    }
    assert!(h.to_bytes().len() > before, "grew");
    assert_equal(&h, "after grow");

    let cold = h.rotate(|id| id.ends_with('2') || id.ends_with('4')).unwrap();
    assert_equal(&h, "after rotate");
    assert_equal(&Hub::load(&cold).unwrap(), "the archive");
    assert!(h.history("ord_11").is_empty() && !Hub::load(&cold).unwrap().history("ord_11").is_empty());
    h.append(EventKind::Paid, "ord_12", r#"{"_d":true,"paid":true}"#, 5000, [0u8; 32]).unwrap();
    assert_equal(&h, "append after rotate");

    let n = h.redact(|e: &Event| e.order_json.contains("pad").then(|| e.order_json.replace("pad", "red"))).unwrap();
    assert!(n > 0, "something was redacted");
    assert_equal(&h, "after redact");
    h.append(EventKind::Paid, "ord_14", r#"{"_d":true,"paid":true}"#, 5001, [0u8; 32]).unwrap();
    assert_equal(&h, "append after redact");
}

fn hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// THE OLD IMAGE, AS A GOLDEN. A deterministic v2 venue (the only version `Hub` has ever
/// written); IMAGE_SHA pins its bytes and HISTORY_SHA every id's history, both taken from
/// the code BEFORE the scan (history() = the walk), by running this with the constants empty.
const IMAGE_SHA: &str = "8be3c3135207a4fb4b88db7ab463e43430dce30191e85d1ab3a7ab75384b224f";
const HISTORY_SHA: &str = "1e5aa0b7f479efdff1043b6b66be9f1bb4fc85e9fbfdbdcbbcda1352b09d1e95";

#[test]
fn an_old_image_answers_history_identically() {
    let h = venue(300, 1200, 0x01D1_3A6E);
    let image = hex(&h.to_bytes_trimmed());
    let back = Hub::load(&h.to_bytes_trimmed()).unwrap();
    let text: String = ids(&back).iter().map(|id| format!("{id:?}={:?};", back.history(id))).collect();
    let hist = hex(text.as_bytes());
    println!("GOLDEN OCHAIN IMAGE_SHA={image} HISTORY_SHA={hist} records={}", back.len());
    assert_eq!(image, IMAGE_SHA, "the image changed by one byte or more");
    assert_eq!(hist, HISTORY_SHA, "history() answered an old image differently");
}
