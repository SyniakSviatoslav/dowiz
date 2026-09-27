//! The orders projection against the fold it replaces (`hubstore::orders_state`)
//! and against `rebuild`, over REAL hubs. Named sequences only -- no fuzzers in
//! this repo.

use super::*;
use crate::fold::{delta, DELTA_MARK};
use dowiz_hub::{EventKind, Hub};
use serde_json::json;

/// What the object's old memo served: `orders_state`, as views.
fn refold(hub: &Hub) -> Vec<OrderView> {
    crate::hubstore::orders_state(hub).into_iter().map(OrderView::of).collect()
}

/// Append to the hub AND build the event the way `hubdo::append` builds it.
fn put(hub: &mut Hub, kind: EventKind, id: &str, payload: &str, seq: u64) -> Event {
    hub.append(kind, id, payload, seq, [0u8; 32]).unwrap();
    let e = Event { kind, order_id: id.into(), order_json: payload.into(), seq };
    assert_eq!(hub.events()[0], e, "the object's hand-built event is the one the log holds");
    e
}

fn warm(hub: &Hub, generation: i64) -> Option<Orders> {
    Some(Orders::of(generation, &hub.events()))
}

fn at(memo: &Option<Orders>) -> Option<i64> {
    memo.as_ref().map(Orders::generation)
}

fn placed(id: &str) -> String {
    json!({"id": id, "status": "PENDING", "total": 2650, "items": [{"product_id": "maki", "quantity": 2}],
           "contact": {"name": "Ana Hoxha", "phone": "+355691234567"}})
    .to_string()
}

fn d(fields: serde_json::Value) -> String {
    let mut v = fields;
    v[DELTA_MARK] = json!(true);
    v.to_string()
}

/// THE WHOLE FOLD IS THE OLD ONE: same orders, same order, same kind and seq,
/// null folds dropped, non-order events skipped.
#[test]
fn a_whole_fold_serves_exactly_what_orders_state_served() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    put(&mut h, EventKind::Placed, "o1", &placed("o1"), 1);
    put(&mut h, EventKind::Placed, "o2", &placed("o2"), 2);
    put(&mut h, EventKind::Revealed, "cust:1", r#"{"who":"owner"}"#, 3);
    put(&mut h, EventKind::Advanced, "o1", &d(json!({"status": "CONFIRMED"})), 4);
    put(&mut h, EventKind::Placed, "o3", "not json", 5); // folds to null: not served
    put(&mut h, EventKind::Advanced, "o2", "null", 6); // skipped, but o2 is now newest
    let o = Orders::of(9, &h.events());
    assert_eq!(o.view(), refold(&h));
    assert_eq!(o.view().iter().map(|v| v.order_id.as_str()).collect::<Vec<_>>(), ["o2", "o1"]);
    assert_eq!(o.generation(), 9);
    assert_eq!(o.pairs()[1], ("o1".into(), refold(&h)[1].order_json.clone()));
}

/// (a) A DELTA HISTORY AND A SNAPSHOT HISTORY give the same projection after N
/// appends, each applied through the memo -- `fold.rs`'s property, now through
/// the incremental path.
#[test]
fn a_delta_history_and_a_snapshot_history_step_to_the_same_projection() {
    let mut full = Hub::create_sized(64 * 1024).unwrap();
    let mut deltas = Hub::create_sized(64 * 1024).unwrap();
    let (mut mf, mut md) = (warm(&full, 0), warm(&deltas, 0));
    let mut states: Vec<serde_json::Value> = Vec::new();
    let mut g = 0;
    for round in 0..6 {
        for n in 0..5usize {
            let id = format!("o{n}");
            let (kind, next) = if round == 0 {
                (EventKind::Placed, serde_json::from_str(&placed(&id)).unwrap())
            } else {
                let mut s: serde_json::Value = states[n].clone();
                s["status"] = json!(["CONFIRMED", "COOKING", "READY", "IN_DELIVERY", "DELIVERED"][round - 1]);
                s[format!("t{round}")] = json!(round * 1000 + n);
                if round == 3 {
                    s.as_object_mut().unwrap().remove("contact");
                }
                (EventKind::Advanced, s)
            };
            let seq = (g + 1) as u64;
            let ef = put(&mut full, kind, &id, &next.to_string(), seq);
            let dp = if round == 0 { next.to_string() } else { delta(&states[n], &next).to_string() };
            let ed = put(&mut deltas, kind, &id, &dp, seq);
            after_log_write(&mut mf, g, g + 1, Written::Appended(ef));
            after_log_write(&mut md, g, g + 1, Written::Appended(ed));
            if round == 0 { states.push(next) } else { states[n] = next }
            g += 1;
        }
    }
    let (vf, vd) = (mf.unwrap().view(), md.unwrap().view());
    assert_eq!(vf.len(), 5);
    for (a, b) in vf.iter().zip(&vd) {
        assert_eq!((&a.order_id, a.kind, a.seq), (&b.order_id, b.kind, b.seq));
        let (ja, jb): (serde_json::Value, serde_json::Value) =
            (serde_json::from_str(&a.order_json).unwrap(), serde_json::from_str(&b.order_json).unwrap());
        assert_eq!(ja, jb, "{}", a.order_id);
        assert!(ja.get("contact").is_none(), "the dropped field stays dropped");
    }
    assert_eq!(vd, refold(&deltas), "and the delta memo is what a fresh fold serves");
}

/// The hundred named steps of test (b): (kind, target order, payload).
fn script(i: usize) -> (EventKind, String, String) {
    let r = i / 10;
    let me = format!("o{r}");
    let prev = format!("o{}", r.saturating_sub(1)); // touching an older order reorders
    match i % 10 {
        0 => (EventKind::Placed, me.clone(), placed(&me)),
        1 => (EventKind::Advanced, me, d(json!({"status": "CONFIRMED"}))),
        2 => (EventKind::Noted, prev, d(json!({"courier_id": format!("c{r}")}))),
        3 => (EventKind::Paid, me, d(json!({"amount_received": 2650}))),
        4 => (EventKind::Advanced, me, d(json!({"status": "PREPARING", "_x": ["contact"]}))),
        5 => (EventKind::Revealed, format!("cust:{r}"), r#"{"who":"owner"}"#.into()),
        6 => (EventKind::Amended, me, d(json!({"items": [{"product_id": "maki", "quantity": 1}], "total": 1325}))),
        7 => (EventKind::Advanced, me, "not json".into()),
        8 => (EventKind::Advanced, prev, d(json!({"status": "READY", "note": null}))),
        _ => (EventKind::Advanced, me.clone(), json!({"id": me, "status": "DELIVERED"}).to_string()),
    }
}

/// (b) `rebuild` AFTER 100 APPENDS REPORTS NOTHING STALE -- and the memo it
/// compared was really the stepped one, never dropped and refolded (a dropped
/// memo would pass this vacuously). Every third step lands as a command's
/// whole hub (`Written::Log`), the rest as appends.
#[test]
fn rebuild_after_a_hundred_named_appends_finds_nothing_stale() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let mut memo = warm(&h, 0);
    for i in 0..100 {
        let (kind, id, payload) = script(i);
        let e = put(&mut h, kind, &id, &payload, i as u64 + 1);
        let written = if i % 3 == 0 { Written::Log(h.events()) } else { Written::Appended(e) };
        after_log_write(&mut memo, i as i64, i as i64 + 1, written);
        assert_eq!(at(&memo), Some(i as i64 + 1), "step {i} must step, not drop");
    }
    let r = crate::rebuild::of_log(&h, memo.as_ref(), 100, &[], false);
    assert_eq!(r.stale, Vec::<String>::new(), "{r:?}");
    assert_eq!(r.orders, 10);
    assert_eq!(memo.unwrap().view(), refold(&h));
}

/// (c) A WHOLE-IMAGE WRITE DROPS THE MEMO, and the next read refolds -- once.
/// Twin: after an append the read is served from the memo, no load at all.
#[test]
fn a_whole_image_write_drops_the_memo_and_the_next_read_refolds() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    put(&mut h, EventKind::Placed, "o1", &placed("o1"), 1);
    let mut memo = None;
    let loads = std::cell::Cell::new(0);
    let load = |h: &Hub| {
        loads.set(loads.get() + 1);
        Ok::<_, ()>(h.events())
    };
    assert_eq!(read(&mut memo, 1, || load(&h)).unwrap(), refold(&h), "cold: folds");
    assert_eq!(loads.get(), 1);
    let e = put(&mut h, EventKind::Advanced, "o1", &d(json!({"status": "CONFIRMED"})), 2);
    after_log_write(&mut memo, 1, 2, Written::Appended(e));
    assert_eq!(read(&mut memo, 2, || load(&h)).unwrap(), refold(&h), "stepped: served");
    assert_eq!(loads.get(), 1, "an append costs the next read no fold");

    after_log_write(&mut memo, 2, 3, Written::Whole);
    assert!(memo.is_none(), "a whole-image write drops the memo");
    assert_eq!(read(&mut memo, 3, || load(&h)).unwrap(), refold(&h));
    assert_eq!(loads.get(), 2, "and the next read refolds");
    assert_eq!(at(&memo), Some(3));
    // A memo at another generation is not served either.
    read(&mut memo, 4, || load(&h)).unwrap();
    assert_eq!(loads.get(), 3);
    // A failed load leaves nothing behind.
    assert_eq!(read(&mut None, 5, || Err::<Vec<Event>, _>("unreadable")), Err("unreadable"));
}

/// `forget` REWRITES OLD RECORDS IN PLACE through the same door as a command.
/// A memo that stepped over it would keep serving the forgotten customer, so
/// a changed history drops it. Twin: an untouched history steps.
#[test]
fn a_command_that_rewrote_history_drops_the_memo_and_one_that_did_not_steps() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    put(&mut h, EventKind::Placed, "o1", &placed("o1"), 1);
    put(&mut h, EventKind::Placed, "o2", &placed("o2"), 2);
    let base = h.events();

    let mut appended = h.events();
    appended.insert(0, Event { kind: EventKind::Noted, order_id: "o1".into(), order_json: d(json!({"k": 1})), seq: 3 });
    appended.insert(0, Event { kind: EventKind::Forgotten, order_id: "cust:1".into(), order_json: "{}".into(), seq: 4 });
    let mut memo = Some(Orders::of(7, &base));
    after_log_write(&mut memo, 7, 8, Written::Log(appended));
    assert_eq!(at(&memo), Some(8), "two new events on an untouched history step");
    assert_eq!(memo.as_ref().unwrap().view()[0].order_id, "o1", "o1 moved to the front");

    let mut redacted = base.clone();
    redacted[1].order_json = placed("o1").replace("Ana Hoxha", "[forgotten]");
    redacted.insert(0, Event { kind: EventKind::Forgotten, order_id: "cust:1".into(), order_json: "{}".into(), seq: 3 });
    let mut memo = Some(Orders::of(7, &base));
    after_log_write(&mut memo, 7, 8, Written::Log(redacted));
    assert!(memo.is_none(), "a redacted history is refolded, never stepped over");

    let mut memo = Some(Orders::of(7, &base));
    after_log_write(&mut memo, 7, 8, Written::Log(base[1..].to_vec()));
    assert!(memo.is_none(), "a log shorter than the memo is not an append");
}

/// A STEP FROM ANOTHER GENERATION IS NOT A STEP: the memo is dropped rather
/// than moved onto a base it never folded. A cold memo stays cold.
#[test]
fn a_write_from_a_generation_the_memo_is_not_at_drops_it() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let e = put(&mut h, EventKind::Placed, "o1", &placed("o1"), 1);
    let mut o = Orders::of(4, &[]);
    assert!(!o.step(3, 5, std::slice::from_ref(&e)), "refused");
    assert_eq!((o.generation(), o.view().len()), (4, 0), "and nothing applied");
    let mut memo = Some(Orders::of(4, &[]));
    after_log_write(&mut memo, 3, 4, Written::Appended(e.clone()));
    assert!(memo.is_none());
    let mut memo = Some(Orders::of(4, &[]));
    after_log_write(&mut memo, 3, 4, Written::Log(h.events()));
    assert!(memo.is_none());
    let mut cold: Option<Orders> = None;
    after_log_write(&mut cold, 0, 1, Written::Appended(e));
    assert!(cold.is_none());
}

/// `rebuild` NOW SEES AN EMPTY SERVED MEMO. The old check read "empty" as
/// "cold" and compared the fresh fold with itself. Twin: a memo at another
/// generation is not being served, and the check stays silent.
#[test]
fn rebuild_names_an_empty_memo_served_over_a_log_with_orders() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    put(&mut h, EventKind::Placed, "o1", &placed("o1"), 1);
    let empty = Orders::of(1, &[]);
    let r = crate::rebuild::of_log(&h, Some(&empty), 1, &[], false);
    assert_eq!(r.stale, vec!["o1"]);
    assert!(crate::rebuild::of_log(&h, Some(&empty), 2, &[], false).intact());
    assert!(crate::rebuild::of_log(&h, None, 1, &[], false).intact());
}

/// MEASURED, not asserted (`cargo test --release --lib measure_projection --
/// --ignored --nocapture`): what one placement pays for the orders projection
/// at 8k orders / 32k events, before (a refold) and after (a step).
#[test]
#[ignore]
fn measure_projection_step_against_the_refold() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    for k in 0..8_000u64 {
        let id = format!("ord_{k}");
        put_fast(&mut h, EventKind::Placed, &id, &placed(&id), 4 * k + 1);
        for (n, st) in ["CONFIRMED", "PREPARING", "DELIVERED"].iter().enumerate() {
            put_fast(&mut h, EventKind::Advanced, &id, &d(json!({"status": st})), 4 * k + 2 + n as u64);
        }
    }
    let us = |t: std::time::Instant| t.elapsed().as_micros();
    let t = std::time::Instant::now();
    for _ in 0..5 {
        std::hint::black_box(refold(&h));
    }
    let before = us(t) / 5;
    let t = std::time::Instant::now();
    let mut memo = Some(Orders::of(0, &h.events()));
    let cold = us(t);
    let (mut step, mut view, mut walk, mut logstep) = (0, 0, 0, 0);
    for k in 0..10u64 {
        let id = format!("new_{k}");
        h.append(EventKind::Placed, &id, &placed(&id), 40_000 + k, [0u8; 32]).unwrap();
        let e = Event { kind: EventKind::Placed, order_id: id.clone(), order_json: placed(&id), seq: 40_000 + k };
        let g = 2 * k as i64;
        let t = std::time::Instant::now();
        after_log_write(&mut memo, g, g + 1, Written::Appended(e));
        step += us(t);
        let t = std::time::Instant::now();
        std::hint::black_box(memo.as_ref().unwrap().view());
        view += us(t);
        h.append(EventKind::Advanced, &id, &d(json!({"status": "CONFIRMED"})), 50_000 + k, [0u8; 32]).unwrap();
        let t = std::time::Instant::now();
        let evs = h.events();
        walk += us(t);
        let t = std::time::Instant::now();
        after_log_write(&mut memo, g + 1, g + 2, Written::Log(evs));
        logstep += us(t);
    }
    assert_eq!(at(&memo), Some(20));
    println!(
        "orders=8000 hub_events={}: BEFORE refold(orders_state)={before}us | AFTER append_step={}us command_walk={}us command_digest+step={}us view_clone={}us | cold Orders::of={cold}us",
        h.len(), step / 10, walk / 10, logstep / 10, view / 10
    );
}

fn put_fast(hub: &mut Hub, kind: EventKind, id: &str, payload: &str, seq: u64) {
    hub.append(kind, id, payload, seq, [0u8; 32]).unwrap();
}
