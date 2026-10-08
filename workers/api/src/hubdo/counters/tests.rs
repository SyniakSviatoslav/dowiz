//! AX0-COUNTERS over the object itself (MemHost, W-COV C2) and through `/api/owner/health`.
//! Three of these are the row's RED proofs: the read-only wake, the wake a hibernation erased
//! (health says so), and the catalogue write's cost on the journal path. The last test is the
//! MEASURE the card asked for: one price edit on a dubin-size menu through `write_image`.

use super::super::host::mem::{Harness, T0};
use crate::wire::Call;
use dowiz_hub::catalog::{edits, Catalog};
use dowiz_hub::logimage::LogImage;
use serde_json::{json, Value};
use worker::Method;

fn placed(id: &str) -> Value {
    json!({
        "kind": dowiz_hub::EventKind::Placed as u8,
        "order_id": id,
        "payload": json!({ "id": id, "status": "PENDING", "total": 1200 }).to_string(),
        "clock": T0 as u64,
    })
}

fn append(h: &Harness, generation: i64, id: &str) {
    let c = Call::new("https://hub/fold/append", Method::Post).unwrap().with_header("x-generation", &generation.to_string()).with_json(&placed(id));
    assert_eq!(h.call(c).status_code(), 200);
}

fn counters(h: &Harness, flush: bool) -> Value {
    h.get(&format!("/fold/counters?flush={}", u8::from(flush))).body_value()
}

/// RED-able (plan §A0's test): a fresh object answering two `/fold/orders` and no write is a
/// read-only wake; one append makes it a writing one.
#[test]
fn wake_then_read_counts_readonly() {
    let h = Harness::new();
    append(&h, 0, "o1");
    let cold = h.cold();
    for _ in 0..2 {
        assert_eq!(cold.get("/fold/orders").status_code(), 200);
    }
    let c = counters(&cold, false);
    assert_eq!((c["wakes_total"].as_u64(), c["wakes_readonly"].as_u64()), (Some(1), Some(1)), "{c}");
    assert_eq!(c["reads"], 3, "two folds and the counters read itself: {c}");
    assert_eq!((c["cold_folds"].as_u64(), c["cold_fold_samples"].as_array().map(Vec::len)), (Some(1), Some(1)), "the second read is the memo: {c}");
    assert!(c["cold_fold_first_us"].is_u64(), "{c}");
    // Positive twin: one write ends the read-only run; its chunks are `proj_rows`.
    append(&cold, 1, "o2");
    let c = counters(&cold, false);
    assert_eq!((c["wakes_total"].as_u64(), c["wakes_readonly"].as_u64(), c["writes"].as_u64()), (Some(1), Some(0), Some(1)), "{c}");
    assert!(c["proj_rows"].as_u64().is_some_and(|n| n >= 1), "a write stores at least one chunk: {c}");
}

/// RED-able: a catch-up the window answers, and one it cannot (`None`, "ask for the list").
#[test]
fn since_counts_the_polls_the_window_could_not_answer() {
    let h = Harness::new();
    append(&h, 0, "o1");
    append(&h, 1, "o2");
    assert_eq!(h.get("/fold/changes?since=1").body_value()["full"], false);
    assert_eq!(h.get("/fold/changes?since=-5").body_value()["full"], true);
    let c = counters(&h, false);
    assert_eq!((c["since_total"].as_u64(), c["since_none"].as_u64()), (Some(2), Some(1)), "{c}");
}

/// RED-able: COUNTERS THAT A HIBERNATION ERASED ARE SAID TO BE ERASED. The first object counts
/// and is never flushed; the object that replaces it starts at zero, and its window says it began
/// at a WAKE -- so a collector whose last flush is older sees the gap instead of a clean zero. A
/// flushed window says `flush` and holds only what came after it.
#[test]
fn a_counter_erased_by_hibernation_is_shown_as_a_wake_window_not_a_clean_zero() {
    let h = Harness::new();
    append(&h, 0, "o1");
    h.get("/fold/changes?since=0");
    assert_eq!(counters(&h, false)["since_total"], 1);
    let woken = h.cold(); // the eviction: same storage, new object, nothing flushed
    let c = counters(&woken, false);
    assert_eq!(c["since_total"], 0, "the first object's count is gone: {c}");
    assert_eq!(c["window"]["cause"], "wake", "a window that began at a wake must say so: {c}");
    assert_eq!(c["window"]["fromMs"], c["window"]["wokeAtMs"], "{c}");
    // Positive twin: a flush answers, zeroes, and the next window is a FLUSH window without the wake.
    woken.get("/fold/changes?since=0");
    let f = counters(&woken, true);
    assert_eq!((f["since_total"].as_u64(), f["window"]["flushed"].as_bool()), (Some(1), Some(true)), "{f}");
    let after = counters(&woken, false);
    assert_eq!(after["window"]["cause"], "flush", "{after}");
    assert_eq!((after["since_total"].as_u64(), after["wakes_total"].as_u64()), (Some(0), Some(0)), "the wake is counted once: {after}");
    assert_eq!(after["wake"]["reads"].as_u64(), Some(f["wake"]["reads"].as_u64().unwrap() + 1), "the wake's own reads are never flushed: {after}");
}

fn dish(i: usize, price: i64, pad: usize) -> String {
    format!(r#"{{"id":"d{i:03}","name":"Dish {i}","price":{price},"available":true,"description":"{}"}}"#, "x".repeat(pad))
}

fn put_cat(h: &Harness, gen: i64, bytes: &[u8]) -> u16 {
    let c = Call::new("https://hub/img/catalog", Method::Put).unwrap().with_header("x-generation", &gen.to_string())
        .with_header("x-edit-by", "owner-1").with_header("x-edit-at", &T0.to_string()).with_body(bytes.to_vec());
    h.call(c).status_code()
}

/// RED-able: a stamped catalogue write is counted on the journal path -- the decode of the
/// catalogue held AND the one written, and the journal it appended to.
#[test]
fn a_catalogue_write_counts_its_decodes_and_its_journal() {
    let h = Harness::new();
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"name":"Cost","currency":"ALL"}"#);
    c.set_product("d000", &dish(0, 500, 10));
    let first = c.to_bytes().unwrap();
    assert_eq!(put_cat(&h, 0, &first), 200);
    c.set_product("d000", &dish(0, 600, 10));
    let second = c.to_bytes().unwrap();
    assert_eq!(put_cat(&h, 1, &second), 200);
    let k = counters(&h, false);
    assert_eq!(k["cat_writes"], 2, "{k}");
    assert_eq!(k["cat_decoded_bytes"].as_u64(), Some((first.len() + first.len() + second.len()) as u64), "nothing held, then the two: {k}");
    assert!(k["journal_bytes"].as_u64().is_some_and(|n| n > 0), "the second write loaded the journal the first one began: {k}");
    // Positive twin: a write to another image is a write, never a catalogue write.
    assert_eq!(h.put("settings", 0, b"x").status_code(), 200);
    assert_eq!(counters(&h, false)["cat_writes"], 2);
}

/// THE MEASURE (card W-AX0 (e)): one single-price edit on a dubin-size catalogue (165 dishes,
/// ~538 KB) with a 599 -> 600-record journal, through the object's PUT -> `put_image_as` ->
/// `write_image`, natively in MemHost. Printed with `--nocapture` as `AX0-MEASURE`. The pass mark
/// is only that the counters saw the edit; the µs are the deliverable, debug or release as built.
#[test]
fn one_price_edit_on_a_dubin_size_menu_is_timed_by_its_counters() {
    let h = Harness::new();
    let mut c = Catalog::create().unwrap();
    c.set_location(r#"{"name":"Dubin-size","currency":"ALL","menu_version":1}"#);
    c.set_category("rolls", r#"{"id":"rolls","name":"Rolls"}"#);
    for i in 0..165 {
        c.set_product(&format!("d{i:03}"), &dish(i, 900, 3150));
    }
    let base = c.to_bytes().unwrap();
    assert_eq!(put_cat(&h, 0, &base), 200);
    let s0 = edits::state_of(&Catalog::load(&base).unwrap());
    let (mut log, mut cur, toggles) = (LogImage::create().unwrap(), s0.clone(), 432i64);
    for i in 0..toggles {
        let mut next = cur.clone();
        let d = (i / 2) as usize % 165;
        next.insert(format!("product:d{d:03}"), dish(d, if i % 2 == 0 { 901 } else { 900 }, 3150));
        edits::journal(&mut log, &cur, &next, T0 + i, "owner-1", (i - toggles + 1, i - toggles + 2)).unwrap();
        cur = next;
    }
    let jgen = Harness::gen_of(&h.get(&format!("/img/{}", crate::catalog_history::IMAGE)));
    assert_eq!(h.put(crate::catalog_history::IMAGE, jgen, &log.to_bytes()).status_code(), 200);
    let journal_len = h.get(&format!("/img/{}", crate::catalog_history::IMAGE)).body().len();
    let edits_n = 7usize;
    let (mut walls, mut decode, mut journal, mut jbytes) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    counters(&h, true);
    for e in 0..edits_n {
        let mut c = Catalog::load(&h.get("/img/catalog").body().to_vec()).unwrap();
        c.set_product("d007", &dish(7, 1250 + e as i64, 3150));
        let next = c.to_bytes().unwrap();
        counters(&h, true);
        let t = std::time::Instant::now();
        assert_eq!(put_cat(&h, 1 + e as i64, &next), 200);
        walls.push(t.elapsed().as_micros() as u64);
        let k = counters(&h, true);
        assert_eq!(k["cat_writes"], 1, "{k}");
        assert!(k["cat_decoded_bytes"].as_u64().is_some_and(|n| n >= 2 * base.len() as u64), "two catalogues decoded per edit: {k}");
        decode.push(k["cat_decode_us"].as_u64().unwrap());
        journal.push(k["journal_us"].as_u64().unwrap());
        jbytes.push(k["journal_bytes"].as_u64().unwrap());
        println!("AX0-EDIT {e}: PUT {} µs | decode {} µs over {} B | journal load+append {} µs over {} B | {} chunk rows", walls[e], decode[e], k["cat_decoded_bytes"], journal[e], jbytes[e], k["proj_rows"]);
    }
    assert!(jbytes[0] >= journal_len as u64, "the first edit loads the stored journal whole: {jbytes:?}");
    let p50 = |v: &[u64]| { let mut v = v[1..].to_vec(); v.sort_unstable(); v[v.len() / 2] };
    println!(
        "AX0-MEASURE native-us catalogue {} B, journal {} B (599 records) | first edit {} µs | edits 2..{}: PUT p50 {} µs, decode p50 {} µs, journal p50 {} µs over ~{} B | build {}",
        base.len(), journal_len, walls[0], edits_n, p50(&walls), p50(&decode), p50(&journal), p50(&jbytes),
        if cfg!(debug_assertions) { "debug" } else { "release" }
    );
}

/// Through the owner's health route: `counters` is there, and `?counters=flush` flushes.
#[test]
fn health_carries_the_counters_and_flushes_only_when_asked() {
    use crate::edge::site::{get, As, Site, PLATFORM_HOST};
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let url = |q: &str| format!("https://alpha.{PLATFORM_HOST}/api/owner/health{q}");
    let one = site.run(crate::services::operations::health, get(&url("")).bearer(&a).on("alpha"), &[]);
    assert_eq!(one.status_code(), 200, "{}", one.body_str());
    let c = &one.body_value()["counters"];
    assert!(c["since_total"].is_u64() && c["window"]["flushed"] == false, "{c}");
    let f = site.run(crate::services::operations::health, get(&url("?counters=flush")).bearer(&a).on("alpha"), &[]);
    assert_eq!(f.body_value()["counters"]["window"]["flushed"], true, "{}", f.body_str());
}
