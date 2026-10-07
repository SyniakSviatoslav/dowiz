//! The object's own rules, and (W-COV C2) the object itself over memory.

use super::{changed_chunks, OrderView};

/// A CLIENT THAT WAS AWAY IS TOLD WHAT IT MISSED, or told to ask again --
/// and the difference matters more than either answer. An empty list where
/// the window does not reach is a lie shaped exactly like "nothing has
/// changed", and a console would believe it for as long as it stayed open.
#[test]
fn a_catch_up_says_what_changed_or_says_it_cannot() {
    let mk = |g: i64| super::Change {
        generation: g,
        kind: dowiz_hub::EventKind::Advanced as u8,
        order_id: format!("ord_{g}"),
        payload: String::new(),
    };
    let window: Vec<super::Change> = (10..=14).map(mk).collect();

    // Inside the window: only what is newer.
    let got = super::changes_since(&window, 12).expect("the window covers 12");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].generation, 13);
    assert_eq!(got[1].generation, 14);

    // The exact edge: a client at 9 has seen everything before 10.
    assert!(super::changes_since(&window, 9).is_some());
    assert_eq!(super::changes_since(&window, 9).unwrap().len(), 5);

    // Before the edge: the window cannot say, and says so.
    assert!(super::changes_since(&window, 8).is_none());
    assert!(super::changes_since(&window, 0).is_none());

    // Up to date: nothing changed, which is a real answer.
    assert_eq!(super::changes_since(&window, 14).unwrap().len(), 0);

    // A cold object has no window at all.
    assert!(super::changes_since(&[], 14).is_none());
}

/// The projection crosses a boundary as JSON, so its shape is a contract.
/// `kind` travels as the byte the log itself stores, because the enum
/// cannot cross and a name could drift from the number.
#[test]
fn an_order_view_survives_the_json_it_crosses_on() {
    let view = OrderView {
        order_id: "ord_1".into(),
        kind: dowiz_hub::EventKind::Advanced as u8,
        seq: 1789000000000,
        order_json: r#"{"id":"ord_1","status":"COOKING"}"#.into(),
    };
    let wire = serde_json::to_string(&view).expect("serialise");
    let back: OrderView = serde_json::from_str(&wire).expect("parse");
    assert_eq!(back.order_id, view.order_id);
    assert_eq!(back.seq, view.seq);
    assert_eq!(back.order_json, view.order_json);
    assert_eq!(
        dowiz_hub::EventKind::from_u8(back.kind),
        Some(dowiz_hub::EventKind::Advanced),
        "the byte has to name the same kind on the other side"
    );
}


#[test]
fn without_an_old_image_every_chunk_is_written() {
    assert_eq!(changed_chunks(None, &[1u8; 250], 100), vec![0, 1, 2]);
    assert_eq!(changed_chunks(None, &[], 100), vec![0]);
}

#[test]
fn an_identical_image_writes_nothing() {
    let img = [7u8; 250];
    assert_eq!(changed_chunks(Some(&img), &img, 100), Vec::<usize>::new());
}

#[test]
fn an_append_touches_the_front_and_the_tail_only() {
    let mut old = vec![0u8; 250];
    old[3] = 1;
    let mut new = old.clone();
    new[3] = 2; // the superblock moved
    new[249] = 9; // the tail moved
    assert_eq!(changed_chunks(Some(&old), &new, 100), vec![0, 2]);
}

#[test]
fn growth_writes_the_new_chunks_and_the_last_old_one_it_extends() {
    let old = vec![0u8; 250];
    let mut new = vec![0u8; 420];
    new[300] = 1;
    // chunk 2 is now [200,300) where before it was [200,250): a longer
    // slice than storage holds, so it is written; chunks 3 and 4 are
    // wholly new; chunks 0 and 1 are untouched.
    assert_eq!(changed_chunks(Some(&old), &new, 100), vec![2, 3, 4]);
}

#[test]
fn a_shorter_image_rewrites_the_chunk_that_got_shorter() {
    let old = vec![5u8; 420];
    let new = vec![5u8; 250];
    // chunks 0 and 1 are the same 100 bytes; chunk 2 is now 50 bytes
    // where storage holds 100, so it MUST be written even though those
    // 50 bytes match -- the reviewer's case: skip it and the next cold
    // load assembles 300 bytes under a meta that says 250.
    assert_eq!(changed_chunks(Some(&old), &new, 100), vec![2]);
}

// ── THE OBJECT ITSELF, over memory (W-COV C2) ─────────────────────────────
//
// Everything below drives `HubImages::route` -- the code the platform runs --
// with a `MemHost` for storage. Outcomes are read back from what was STORED
// (the keys, a cold object's read), not from the reply alone.

use super::host::mem::{Harness, MemHost, Stored, T0};
use crate::wire::Call;
use serde_json::json;
use worker::Method;

fn placed(id: &str) -> serde_json::Value {
    json!({
        "kind": dowiz_hub::EventKind::Placed as u8,
        "order_id": id,
        "payload": json!({ "id": id, "status": "PENDING", "total": 1200 }).to_string(),
        "clock": T0 as u64,
    })
}

fn append(h: &Harness, generation: i64, ev: &serde_json::Value) -> crate::wire::Reply {
    h.call(
        Call::new("https://hub/fold/append", Method::Post)
            .unwrap()
            .with_header("x-generation", &generation.to_string())
            .with_json(ev),
    )
}

#[test]
fn an_absent_image_is_204_at_generation_zero_and_a_write_makes_generation_one() {
    let h = Harness::new();
    let r = h.get("/img/settings");
    assert_eq!(r.status_code(), 204);
    assert_eq!(Harness::gen_of(&r), 0);

    let w = h.put("settings", 0, b"hello");
    assert_eq!(w.status_code(), 200);
    assert_eq!(Harness::gen_of(&w), 1);

    let r = h.get("/img/settings");
    assert_eq!(r.status_code(), 200);
    assert_eq!(r.body(), b"hello");
    assert_eq!(Harness::gen_of(&r), 1);
    assert_eq!(r.headers().get("content-type").unwrap().as_deref(), Some("application/octet-stream"));
    // Stored as ONE chunk and a meta that says so.
    assert_eq!(h.host.keys("c:settings:"), vec!["c:settings:0".to_string()]);
    assert!(matches!(h.host.kv.borrow().get("m:settings"), Some(Stored::Json(v)) if v["len"] == 5 && v["generation"] == 1));
}

#[test]
fn a_write_at_a_stale_generation_is_409_and_changes_nothing() {
    let h = Harness::new();
    h.put("stock", 0, b"one");
    let stale = h.put("stock", 0, b"two");
    assert_eq!(stale.status_code(), 409);
    assert_eq!(stale.body_str(), "generation moved");
    assert_eq!(h.cold().get("/img/stock").body(), b"one", "storage still holds the first write");
    // Positive twin: the current generation writes.
    assert_eq!(h.put("stock", 1, b"two").status_code(), 200);
    assert_eq!(h.cold().get("/img/stock").body(), b"two");
}

#[test]
fn a_write_without_a_generation_or_a_name_or_with_a_wrong_method_is_refused() {
    let h = Harness::new();
    let no_gen = h.call(Call::new("https://hub/img/catalog", Method::Put).unwrap().with_body(b"x".to_vec()));
    assert_eq!(no_gen.status_code(), 400);
    assert_eq!(no_gen.body_str(), "x-generation is required on a write");
    assert!(h.host.keys("").is_empty(), "nothing was written");
    let no_name = h.get("/img/");
    assert_eq!(no_name.status_code(), 400);
    let wrong = h.call(Call::new("https://hub/img/catalog", Method::Delete).unwrap());
    assert_eq!(wrong.status_code(), 405);
    let nothing = h.get("/fold/nothing-here");
    assert_eq!(nothing.status_code(), 404);
}

/// THE REVIEWER'S CASE, through the object: an image that shrinks must not
/// leave a longer chunk behind, and the chunks past its end are deleted.
#[test]
fn a_big_image_is_chunked_and_a_shrink_leaves_no_stale_tail_for_a_cold_read() {
    let h = Harness::new();
    let big: Vec<u8> = (0..(super::CHUNK * 2 + 100)).map(|i| (i % 251) as u8).collect();
    assert_eq!(h.put("catalog", 0, &big).status_code(), 200);
    assert_eq!(h.host.keys("c:catalog:").len(), 3);
    assert_eq!(h.cold().get("/img/catalog").body(), &big[..], "a cold object assembles all three chunks");

    // Shrink to a prefix of the old bytes: chunk 0 now shorter than stored.
    let small = big[..50].to_vec();
    assert_eq!(h.put("catalog", 1, &small).status_code(), 200);
    assert_eq!(h.host.keys("c:catalog:"), vec!["c:catalog:0".to_string()], "chunks 1 and 2 are gone");
    let cold = h.cold().get("/img/catalog");
    assert_eq!(cold.body(), &small[..]);
    assert_eq!(Harness::gen_of(&cold), 2);
}

/// ONLY THE CHUNKS THAT MOVED are written: an edit in the last chunk writes
/// that chunk and the meta, not the whole image.
#[test]
fn an_edit_in_one_chunk_writes_that_chunk_and_the_meta_only() {
    let h = Harness::new();
    let mut img = vec![1u8; super::CHUNK * 3];
    h.put("log2", 0, &img);
    h.host.writes.borrow_mut().clear();
    img[super::CHUNK * 2 + 7] = 9;
    h.put("log2", 1, &img);
    assert_eq!(*h.host.writes.borrow(), vec!["c:log2:2".to_string(), "m:log2".to_string()]);
}

/// A WRITE THAT FAILS PART-WAY evicts the memory copy: the next read is from
/// storage, which still says the old generation, and the next write's diff is
/// against the truth rather than against what memory hoped had landed.
#[test]
fn a_write_that_fails_part_way_is_an_error_and_the_old_generation_still_reads() {
    let h = Harness::new();
    let old = vec![3u8; super::CHUNK * 2];
    h.put("catalog", 0, &old);
    let mut new = old.clone();
    new[0] = 4;
    new[super::CHUNK] = 4;
    h.host.puts_left.set(Some(1)); // cut after one key: the one atomic call lands nothing (W-ATOMIC, `atomic/tests.rs`)
    let failed = h.try_call(
        Call::new("https://hub/img/catalog", Method::Put).unwrap().with_header("x-generation", "1").with_body(new.clone()),
    );
    assert!(failed.is_err(), "a failed write is an error, not a 200");
    h.host.puts_left.set(None);
    // The same object -- memory evicted -- reads storage: generation 1 still.
    let r = h.get("/img/catalog");
    assert_eq!(Harness::gen_of(&r), 1);
    // The retry diffs against what storage HOLDS (the old image, nothing of
    // the cut write), writes both chunks, and lands.
    assert_eq!(h.put("catalog", 1, &new).status_code(), 200);
    assert_eq!(h.cold().get("/img/catalog").body(), &new[..]);
}

/// AN ERROR IS NOT AN ABSENCE: a store that cannot answer must not be read as
/// "this venue has no image" (which would hand the caller a fresh hub).
#[test]
fn a_storage_error_is_an_error_not_an_empty_image() {
    let h = Harness::new();
    h.put("log", 0, b"x");
    let cold = h.cold();
    cold.host.reads_fail.set(true);
    assert!(cold.try_call(Call::new("https://hub/img/log", Method::Get).unwrap()).is_err());
    assert!(cold.try_call(Call::new("https://hub/fold/orders", Method::Get).unwrap()).is_err());
    cold.host.reads_fail.set(false);
    assert_eq!(cold.get("/img/log").status_code(), 200, "twin: the same read succeeds when storage answers");
}

#[test]
fn a_meta_whose_chunks_are_missing_or_short_is_refused_not_assembled() {
    let host = MemHost::named("v1", T0);
    host.kv.borrow_mut().insert("m:catalog".into(), Stored::Json(json!({"generation": 4, "chunks": 2, "len": 10})));
    host.kv.borrow_mut().insert("c:catalog:0".into(), Stored::Bytes(vec![0; 5]));
    let h = Harness::over(host);
    let e = h.try_call(Call::new("https://hub/img/catalog", Method::Get).unwrap()).err().expect("missing chunk");
    assert!(e.to_string().contains("missing chunk c:catalog:1"), "{e}");
    h.host.kv.borrow_mut().insert("c:catalog:1".into(), Stored::Bytes(vec![0; 4]));
    let e = h.try_call(Call::new("https://hub/img/catalog", Method::Get).unwrap()).err().expect("short");
    assert!(e.to_string().contains("is 9 bytes, its meta says 10"), "{e}");
    h.host.kv.borrow_mut().insert("c:catalog:1".into(), Stored::Bytes(vec![0; 5]));
    assert_eq!(h.get("/img/catalog").body().len(), 10, "twin: a whole image reads");
}

#[test]
fn an_append_lands_is_projected_and_is_told_to_the_right_sockets_only() {
    let h = Harness::new();
    let console = h.host.socket(&[super::TAG_CONSOLE]);
    let mine = h.host.socket(&["order:o1"]);
    let other = h.host.socket(&["order:o2"]);
    let kitchen = h.host.socket(&[crate::services::orders::kitchen_ack::board::TAG_KITCHEN]);

    let r = append(&h, 0, &placed("o1"));
    assert_eq!(r.status_code(), 200);
    assert_eq!(r.body_value(), json!({"generation": 1, "events": 1}));
    assert_eq!(Harness::gen_of(&r), 1);

    // The projection, from a COLD object: it was stored, not just remembered.
    let orders = h.cold().get("/fold/orders");
    assert_eq!(Harness::gen_of(&orders), 1);
    let list = orders.body_value();
    assert_eq!(list.as_array().map(Vec::len), Some(1));
    assert_eq!(list[0]["order_id"], "o1");

    assert_eq!(console.borrow().len(), 1);
    assert_eq!(mine.borrow().len(), 1);
    assert!(other.borrow().is_empty(), "another customer's socket hears nothing");
    let said: serde_json::Value = serde_json::from_str(&console.borrow()[0]).unwrap();
    assert_eq!(said["t"], "event");
    assert_eq!(said["orderId"], "o1");
    assert_eq!(kitchen.borrow().len(), 1, "the pass hears the ticket");
    assert!(!kitchen.borrow()[0].contains("total"), "and never the price: {}", kitchen.borrow()[0]);
}

#[test]
fn an_append_at_a_moved_generation_is_409_and_without_one_is_400() {
    let h = Harness::new();
    append(&h, 0, &placed("o1"));
    let r = append(&h, 0, &placed("o2"));
    assert_eq!(r.status_code(), 409);
    let r = h.call(Call::new("https://hub/fold/append", Method::Post).unwrap().with_json(&placed("o2")));
    assert_eq!(r.status_code(), 400);
    assert_eq!(h.get("/fold/orders").body_value().as_array().map(Vec::len), Some(1), "neither landed");
}

#[test]
fn an_unknown_event_kind_is_an_error_and_writes_nothing() {
    let h = Harness::new();
    let r = h.try_call(
        Call::new("https://hub/fold/append", Method::Post)
            .unwrap()
            .with_header("x-generation", "0")
            .with_json(&json!({"kind": 250, "order_id": "o3", "payload": "{}", "clock": 1})),
    );
    assert!(r.is_err());
    assert!(h.host.keys("m:").is_empty());
}

#[test]
fn one_order_generation_and_changes_answer_from_the_log() {
    let h = Harness::new();
    append(&h, 0, &placed("o1"));
    append(&h, 1, &placed("o2"));

    let one = h.get("/fold/order?id=o2");
    assert_eq!(one.status_code(), 200);
    assert_eq!(one.body_value()["order_id"], "o2");
    assert_eq!(Harness::gen_of(&one), 2);
    assert_eq!(h.get("/fold/order?id=nope").status_code(), 404);
    assert_eq!(h.get("/fold/order").status_code(), 400);

    assert_eq!(h.get("/fold/generation").body_value(), json!({"generation": 2}));

    let c = h.get("/fold/changes?since=1").body_value();
    assert_eq!(c["full"], false);
    assert_eq!(c["changes"].as_array().map(Vec::len), Some(1));
    assert_eq!(c["changes"][0]["order_id"], "o2");
    // A cold object has no window: "ask for the list", never "nothing changed".
    let c = h.cold().get("/fold/changes?since=1").body_value();
    assert_eq!(c["full"], true);
    assert_eq!(c["generation"], 2);
}

/// A WHOLE-IMAGE WRITE TO THE LOG IS A GAP: the window is cleared and every
/// console is told the log moved, so no client believes "nothing changed".
#[test]
fn a_whole_log_write_clears_the_window_and_says_moved() {
    let h = Harness::new();
    append(&h, 0, &placed("o1"));
    let console = h.host.socket(&[super::TAG_CONSOLE]);
    let log = h.get("/img/log").body().to_vec();
    assert_eq!(h.put("log", 1, &log).status_code(), 200);
    assert_eq!(h.get("/fold/changes?since=1").body_value()["full"], true);
    let said: serde_json::Value = serde_json::from_str(&console.borrow()[0]).unwrap();
    assert_eq!(said, json!({"t": "moved", "generation": 2}));
}

#[test]
fn the_venue_record_is_null_until_a_catalogue_names_one() {
    let h = Harness::new();
    let r = h.get("/fold/venue");
    assert_eq!(r.body_str(), "null");
    assert_eq!(r.headers().get("content-type").unwrap().as_deref(), Some("application/json"));
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_location(r#"{"id":"v1","timezone":"Europe/Tirane"}"#);
    h.put("catalog", 0, &cat.to_bytes().unwrap());
    assert_eq!(h.get("/fold/venue").body_value()["timezone"], "Europe/Tirane");
}

/// A MESSAGE FROM A SOCKET IS NOT A PRINCIPAL: a fix is taken only from a
/// socket the Worker tagged `courier:<id>`, and only inside the world.
#[test]
fn a_position_comes_from_the_tag_not_the_frame_and_expires() {
    let h = Harness::new();
    let courier = || vec!["courier".to_string(), "courier:c7".to_string()];
    assert_eq!(h.obj.on_message(r#"{"t":"ping"}"#, Vec::new), Some(r#"{"t":"pong"}"#));
    assert_eq!(h.obj.on_message("not json", Vec::new), None);
    // A console saying it is courier c9 moves nobody.
    h.obj.on_message(r#"{"t":"gps","courier":"c9","lat_e6":41000000,"lng_e6":19000000}"#, || vec!["console".into()]);
    // Out of the world: ignored.
    h.obj.on_message(r#"{"t":"gps","lat_e6":91000000,"lng_e6":19000000}"#, courier);
    h.obj.on_message(r#"{"t":"gps","lat_e6":41000000}"#, courier);
    assert_eq!(h.get("/fold/positions").body_value(), json!({}));
    h.obj.on_message(r#"{"t":"gps","courier":"c9","lat_e6":41000000,"lng_e6":19000000}"#, courier);
    let p = h.get("/fold/positions").body_value();
    assert_eq!(p["c7"]["lat_e6"], 41000000, "attributed to the TAG's courier");
    assert!(p.get("c9").is_none());
    h.host.clock.set(T0 + super::POSITION_KEEP_MS);
    assert_eq!(h.get("/fold/positions").body_value(), json!({}), "twenty minutes on, it is a memory");
}

/// BUG (W-COV, found by this harness): every APPEND was treated as a gap. The
/// log write cleared the catch-up window and sent every console `moved`
/// before the append's own `event`, so (1) the window never held more than one
/// change -- `?since=` one write back answered "ask for the list" -- and (2) a
/// staff console, whose `applyLocally` would have applied the event with no
/// request, was first told `moved` and re-read the whole list on EVERY write
/// (`public/admin/app.js` `onEvent`). An append adds exactly the one event it
/// then broadcasts, so it is not a gap.
#[test]
fn an_append_is_not_a_gap_the_window_keeps_it_and_no_moved_is_sent() {
    let h = Harness::new();
    let console = h.host.socket(&[super::TAG_CONSOLE]);
    append(&h, 0, &placed("o1"));
    append(&h, 1, &placed("o2"));
    let c = h.get("/fold/changes?since=0").body_value();
    assert_eq!(c["full"], false, "two appends are both in the window: {c}");
    assert_eq!(c["changes"].as_array().map(Vec::len), Some(2));
    let frames: Vec<serde_json::Value> =
        console.borrow().iter().map(|m| serde_json::from_str(m).unwrap()).collect();
    assert!(frames.iter().all(|f| f["t"] == "event"), "no `moved` for an append: {frames:?}");
    assert_eq!(frames.len(), 2);
}

// ── THE COMMANDS, through the object ──────────────────────────────────────

fn order_env(id: &str, venue: &str) -> String {
    json!({
        "id": id, "order_id": id, "location_id": venue, "status": "PENDING",
        "items": [{"product_id": "p1", "quantity": 1, "unit_price": 900, "name": "Futomaki"}],
        "subtotal": 900, "total": 900, "payment": "cash",
        "contact": {"name": "C", "phone": "+355690000000"},
        "created_at_ms": T0,
    })
    .to_string()
}

fn place_body(id: &str, venue: &str, notify: Option<&str>) -> serde_json::Value {
    json!({
        "order_id": id, "envelope": order_env(id, venue), "seq": 1, "bom_lines": [],
        "promo": null, "promo_code": null, "subtotal": 900, "fee": 0, "tip": 0,
        "now_ms": T0, "notify_text": notify,
    })
}

fn settings_with(h: &Harness, pairs: &[(&str, &str)]) {
    let mut s = dowiz_hub::settings::Settings::create().unwrap();
    for (k, v) in pairs {
        s.set(k, v);
    }
    let gen = Harness::gen_of(&h.get("/img/settings"));
    assert_eq!(h.put("settings", gen, &s.to_bytes().unwrap()).status_code(), 200);
}

fn outbox_ids(h: &Harness) -> Vec<String> {
    let r = h.get("/img/outbox");
    if r.status_code() == 204 {
        return Vec::new();
    }
    let t = dowiz_hub::table::Table::load(r.body(), crate::outbox::OUTBOX_BYTES).unwrap();
    t.all(crate::outbox::KIND).into_iter().map(|(id, _)| id).collect()
}

#[test]
fn a_placement_writes_the_log_answers_its_generation_and_queues_the_bell_it_was_given() {
    let h = Harness::new();
    settings_with(&h, &[("notify.telegram.chat", "-100123")]);
    let console = h.host.socket(&[super::TAG_CONSOLE]);
    let r = h.post("/fold/place", &place_body("o1", "v1", Some("NEW ORDER o1")));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(Harness::gen_of(&r), 1);
    let out = r.body_value();
    assert_eq!(out["generation"], 1);
    let stored: serde_json::Value = serde_json::from_str(out["stored"].as_str().unwrap()).unwrap();
    assert_eq!(stored["total"], 900);
    // Stored, read cold.
    let one = h.cold().get("/fold/order?id=o1");
    assert_eq!(one.status_code(), 200);
    // The bell is owed and queued in the same turn.
    assert_eq!(outbox_ids(&h).len(), 1, "one telegram entry: {:?}", outbox_ids(&h));
    assert!(console.borrow().iter().any(|m| m.contains("\"orderId\":\"o1\"")));
    // Twin: without a text, no bell.
    h.post("/fold/place", &place_body("o2", "v1", None));
    assert_eq!(outbox_ids(&h).len(), 1);
}

#[test]
fn a_venue_with_nowhere_to_ring_queues_nothing() {
    let h = Harness::new();
    let r = h.post("/fold/place", &place_body("o1", "v1", Some("NEW ORDER")));
    assert_eq!(r.status_code(), 200);
    assert!(outbox_ids(&h).is_empty());
}

#[test]
fn a_placement_whose_body_does_not_parse_is_an_error_and_writes_nothing() {
    let h = Harness::new();
    let r = h.try_call(Call::new("https://hub/fold/place", Method::Post).unwrap().with_body(b"{not json".to_vec()));
    assert!(r.is_err());
    assert!(h.host.keys("m:").is_empty());
}

fn advance_body(id: &str, venue: &str, next: &str) -> serde_json::Value {
    json!({"order_id": id, "location_id": venue, "next": next, "reason": null, "now_ms": T0 + 60_000})
}

#[test]
fn an_advance_moves_the_order_and_another_venues_owner_gets_not_found() {
    let h = Harness::new();
    h.post("/fold/place", &place_body("o1", "v1", None));
    // Cross-venue: an owner authorised for v2 names v1's order.
    let r = h.post("/fold/advance", &advance_body("o1", "v2", "CONFIRMED"));
    assert_eq!(r.status_code(), 404, "{}", r.body_str());
    assert_eq!(h.get("/fold/generation").body_value()["generation"], 1, "nothing logged for the refusal");
    // Twin: the order's own venue moves it.
    let r = h.post("/fold/advance", &advance_body("o1", "v1", "CONFIRMED"));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["generation"], 2);
    let o = h.cold().get("/fold/order?id=o1").body_value();
    let state: serde_json::Value = serde_json::from_str(o["order_json"].as_str().unwrap()).unwrap();
    assert_eq!(state["status"], "CONFIRMED");
    // An illegal edge is the kernel's 409.
    let r = h.post("/fold/advance", &advance_body("o1", "v1", "PENDING"));
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert_eq!(h.post("/fold/advance", &advance_body("ghost", "v1", "CONFIRMED")).status_code(), 404);
}

#[test]
fn an_assignment_names_the_courier_once_and_refuses_another_venue() {
    let h = Harness::new();
    assert_eq!(
        h.post("/fold/assign", &json!({"order_id": "o1", "location_id": "v1", "courier_id": "c1", "now_ms": T0})).status_code(),
        404,
        "no log, no order"
    );
    h.post("/fold/place", &place_body("o1", "v1", None));
    h.post("/fold/advance", &advance_body("o1", "v1", "CONFIRMED"));
    let other = h.post("/fold/assign", &json!({"order_id": "o1", "location_id": "v2", "courier_id": "c1", "now_ms": T0}));
    assert_eq!(other.status_code(), 404, "{}", other.body_str());
    let r = h.post("/fold/assign", &json!({"order_id": "o1", "location_id": "v1", "courier_id": "c1", "now_ms": T0}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let o = h.cold().get("/fold/order?id=o1").body_value();
    assert!(o["order_json"].as_str().unwrap().contains("c1"), "the order names its courier: {o}");
    let again = h.post("/fold/assign", &json!({"order_id": "o1", "location_id": "v1", "courier_id": "c2", "now_ms": T0}));
    assert_eq!(again.status_code(), 409, "a second courier is refused: {}", again.body_str());
}

#[test]
fn the_kitchen_ack_is_noted_once_and_not_across_venues() {
    let h = Harness::new();
    h.post("/fold/place", &place_body("o1", "v1", None));
    let body = |v: &str| json!({"order_id": "o1", "location_id": v, "by": "k1", "now_ms": T0 + 5});
    assert_eq!(h.post("/fold/kitchen_ack", &body("v2")).status_code(), 404);
    let r = h.post("/fold/kitchen_ack", &body("v1"));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["fresh"], true);
    let r = h.post("/fold/kitchen_ack", &body("v1"));
    assert_eq!(r.body_value()["fresh"], false, "seen twice is still seen once");
}

#[test]
fn the_rebuild_of_an_empty_and_a_used_log() {
    let h = Harness::new();
    assert_eq!(h.get("/fold/rebuild").status_code(), 200);
    h.post("/fold/place", &place_body("o1", "v1", None));
    let r = h.get("/fold/rebuild").body_value();
    assert!(r.is_object(), "{r}");
}
