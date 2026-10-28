//! SHEAF S2b through the C surface's own functions (`--features tablet`): a tablet reads the
//! kitchen board, takes taps offline (the order machine deciding each), sends its tail, and the
//! hub rebases it with `dowiz_hub::glue::orders::rebase` -- every tap merged or held. Then the
//! `extern "C"` export, as a host calls it.
#![cfg(feature = "tablet")]

use bebop_wasm::tablet::{self, bw_tab_order, NOTHING, OK, ORDER};
use dowiz_hub::glue::orders::{read_tail, rebase, Kind, Op};
use dowiz_hub::{EventKind, Hub};
use serde_json::{json, Value};

fn hub() -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    for (id, st) in [("a1", "CONFIRMED"), ("a2", "PENDING")] {
        let o = json!({"id": id, "status": st, "location_id": "v1"}).to_string();
        h.append(EventKind::Placed, id, &o, 1, [0; 32]).unwrap();
    }
    h
}

fn write(h: &mut Hub, op: &Op) -> Result<(), String> {
    let body = match &op.kind {
        Kind::Ack => json!({"_d": true, "kitchen": {"seen": {"at": op.at}}}),
        Kind::Advance { next, .. } => json!({"_d": true, "status": next}),
    };
    h.append(EventKind::Advanced, &op.order, &body.to_string(), 50 + h.events().len() as u64, [0; 32]).map(|_| ()).map_err(|e| format!("{e:?}"))
}

const BOARD: &str = r#"[{"id":"a1","status":"CONFIRMED"},{"id":"a2","status":"PENDING"}]"#;

#[test]
fn taps_offline_then_one_sync_merges_or_holds_every_one() {
    let rail = tablet::order_rail_json("", BOARD, 0).unwrap();
    let (rail, s1) = tablet::order_json(&rail, "a1", "PREPARING", 10).unwrap();
    let (rail, s2) = tablet::order_json(&rail, "a2", "", 11).unwrap();
    let (rail, s3) = tablet::order_json(&rail, "a2", "", 12).unwrap();
    assert_eq!((s1, s2, s3), (OK, OK, NOTHING), "the second ack has nothing to do");
    let e = tablet::order_json(&rail, "a2", "READY", 13).unwrap_err();
    assert_eq!(e.0, ORDER);
    assert!(e.1.starts_with("IllegalTransitionError"), "the order machine's own words: {}", e.1);
    let body: Value = serde_json::from_str(&tablet::order_tail_json(&rail, "kitchen").unwrap()).unwrap();
    let tail: Vec<String> = body["tail"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert_eq!(tail.len(), 2);
    let mut h = hub();
    let ops = read_tail(&tail).unwrap();
    let out = rebase(&mut h, "v1", &ops, write);
    assert!(out.law(tail.len()));
    assert_eq!((out.merged, out.held.len()), (2, 0));
    let after = tablet::order_rail_json(&rail, r#"[{"id":"a1","status":"PREPARING"},{"id":"a2","status":"PENDING","kitchen":{"seen":{"at":11}}}]"#, 2).unwrap();
    let v: Value = serde_json::from_str(&after).unwrap();
    assert_eq!(v["queue"].as_array().unwrap().len(), 0, "what was sent is the hub's now");
}

#[test]
fn the_export_answers_through_out() {
    let rail = tablet::order_rail_json("", BOARD, 0).unwrap();
    let (order, next) = ("a1", "PREPARING");
    let mut out = [0usize; 2];
    let st = unsafe { bw_tab_order(rail.as_ptr(), rail.len(), order.as_ptr(), order.len(), next.as_ptr(), next.len(), 7, out.as_mut_ptr()) };
    assert_eq!(st, OK);
    let got = unsafe { Box::from_raw(core::slice::from_raw_parts_mut(out[0] as *mut u8, out[1])) }.to_vec();
    let v: Value = serde_json::from_slice(&got).unwrap();
    assert_eq!(v["tickets"][0]["status"], "PREPARING");
    let bad = "{";
    let st = unsafe { bw_tab_order(bad.as_ptr(), bad.len(), order.as_ptr(), order.len(), next.as_ptr(), next.len(), 7, out.as_mut_ptr()) };
    assert_eq!(st, tablet::BAD_INPUT, "a rail that is not JSON is refused, not reset");
    unsafe { drop(Box::from_raw(core::slice::from_raw_parts_mut(out[0] as *mut u8, out[1]))) };
}
