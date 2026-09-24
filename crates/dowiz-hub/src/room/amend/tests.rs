//! The amendment over the hub's own images. The Worker's
//! `command/amend/tests.rs` holds every refusal of G3 against this same code;
//! these pin what only this crate can say: the image round-trip, and the
//! offline replay of two tablets selling the last portion.

use super::*;
use crate::stock::{StockEvent, StockLog};
use crate::{EventKind, Hub};

const NOW: i64 = 1_790_000_000_000;

fn dish(pid: &str, supply: &str) -> (String, String) {
    (pid.into(), json!({ "id": pid, "bom": [{ "supply": supply, "qty": 1 }] }).to_string())
}

/// Two open rounds at v1, neither holding anything on the shelf.
fn log() -> Hub {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    for (i, id) in ["r1", "r2"].iter().enumerate() {
        let r = json!({"id": id, "status": "PENDING", "location_id": "v1", "tip": 0, "delivery_fee": 0,
                       "items": [{"product_id": "tea", "quantity": 1, "unit_price": 200}],
                       "subtotal": 200, "discount": 0, "total": 200});
        h.append(EventKind::Placed, id, &r.to_string(), 100 + i as u64, [0; 32]).unwrap();
    }
    h
}

/// ONE portion of the last cake on the shelf.
fn shelf() -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append(&StockEvent::Received { item: "cake".into(), qty: 1 }).unwrap();
    s
}

fn sell_cake(order_id: &str) -> AmendIn {
    AmendIn {
        order_id: order_id.into(), location_id: "v1".into(), base_seq: 0,
        ops: vec![Op::Add { line: json!({"product_id": "cake", "quantity": 1, "unit_price": 450}) }],
        by: "p1".into(), reason: None, may_void: false, boms: vec![dish("cake", "cake")], now_ms: NOW,
    }
}

/// Decide over IMAGES, as a tablet and the object both do: load, decide, trim.
fn over_images(log: &[u8], stock: &[u8], input: &AmendIn) -> Result<(Vec<u8>, Vec<u8>), Refused> {
    let mut h = Hub::load(log).unwrap();
    let mut s = StockLog::load(stock).unwrap();
    let cur = crate::room::view::current(&h, &input.order_id);
    decide(&mut h, &mut s, cur.as_ref(), input)?;
    Ok((h.to_bytes_trimmed(), s.to_bytes_trimmed()))
}

#[test]
fn an_added_line_is_one_amended_delta_and_one_reservation() {
    let (mut h, mut s) = (log(), shelf());
    let cur = crate::room::view::current(&h, "r1");
    let (round, body, seq) = decide(&mut h, &mut s, cur.as_ref(), &sell_cake("r1")).expect("the cake is there");
    assert_eq!(round["total"], json!(650));
    assert_eq!(seq, NOW as u64);
    let d: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(d["_d"], json!(true), "a delta, not a snapshot");
    assert_eq!(d["subtotal"], json!(650));
    let last = h.events().into_iter().next().unwrap();
    assert_eq!((last.kind, last.order_json), (EventKind::Amended, body));
    assert_eq!(s.ledger().unwrap().available("cake"), 0, "the portion is held for r1");
}

#[test]
fn an_order_the_log_does_not_hold_is_not_found_and_writes_nothing() {
    let (mut h, mut s) = (log(), shelf());
    let before = h.len();
    assert_eq!(decide(&mut h, &mut s, None, &sell_cake("r9")), Err(Refused::NotFound));
    assert_eq!(h.len(), before);
}

/// THE REPLAY (D7 CHECK). Two tablets are offline with the same images; each
/// sells the last cake to its own table and each, alone, is right. When they
/// reconnect the object replays their inputs in arrival order against ONE
/// shelf: the first lands, the second is `OutOfStock`, and the shelf holds
/// exactly one reservation.
#[test]
fn two_offline_tablets_selling_the_last_cake_replay_to_one_sale() {
    let (log0, shelf0) = (log().to_bytes_trimmed(), shelf().to_bytes_trimmed());
    let a = over_images(&log0, &shelf0, &sell_cake("r1"));
    let b = over_images(&log0, &shelf0, &sell_cake("r2"));
    assert!(a.is_ok() && b.is_ok(), "offline, each tablet sold the cake it saw");

    let (log1, shelf1) = over_images(&log0, &shelf0, &sell_cake("r1")).expect("the first to arrive lands");
    let second = over_images(&log1, &shelf1, &sell_cake("r2"));
    let Err(Refused::Stock(why)) = second else { panic!("the second sale must be OutOfStock, got {second:?}") };
    assert!(why.contains("cake"), "the refusal names the dish's supply: {why}");

    let s = StockLog::load(&shelf1).unwrap();
    let held: Vec<_> = s.events().into_iter().filter(|e| matches!(e, StockEvent::Reserved { .. })).collect();
    assert_eq!(held.len(), 1, "nothing is reserved twice: {held:?}");
    assert_eq!(s.ledger().unwrap().available("cake"), 0);
    let h = Hub::load(&log1).unwrap();
    assert_eq!(h.events().iter().filter(|e| e.kind == EventKind::Amended).count(), 1, "one sale on the log");
}

/// D8 (G4): a 1200 round with 1000 paid, a line voided so it costs 1000.
/// It IS paid; before the fix `payment_status` stayed null, the pay screen
/// showed 0 owed, and any further payment was refused ("exceeds the total").
#[test]
fn an_amend_down_to_exactly_the_amount_paid_stamps_the_round_paid() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let r = json!({"id": "r1", "status": "CONFIRMED", "location_id": "v1", "tip": 0, "delivery_fee": 0,
                   "items": [{"product_id": "a", "quantity": 1, "unit_price": 1000},
                             {"product_id": "b", "quantity": 1, "unit_price": 200}],
                   "subtotal": 1200, "discount": 0, "total": 1200,
                   "payments": [{"by": "p1", "amount": 1000, "method": "card", "at": 1}]});
    h.append(EventKind::Placed, "r1", &r.to_string(), 100, [0; 32]).unwrap();
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    let cur = crate::room::view::current(&h, "r1").unwrap();
    let void = AmendIn {
        order_id: "r1".into(), location_id: "v1".into(), base_seq: cur.seq, ops: vec![Op::Remove { line: 1 }],
        by: "p1".into(), reason: Some("mistake".into()), may_void: false, boms: vec![], now_ms: NOW,
    };
    let (o, body, _) = decide(&mut h, &mut s, Some(&cur), &void).expect("the void lands");
    assert_eq!((o["total"].clone(), crate::room::pay::paid_of(&o)), (json!(1000), 1000));
    assert_eq!(o["payment_status"], json!("paid"), "{body}");
}

/// The twin: voided down to MORE than was paid, the round is still owed.
#[test]
fn an_amend_that_still_leaves_money_owed_does_not_stamp_paid() {
    let mut h = Hub::create_sized(64 * 1024).unwrap();
    let r = json!({"id": "r1", "status": "CONFIRMED", "location_id": "v1", "tip": 0, "delivery_fee": 0,
                   "items": [{"product_id": "a", "quantity": 1, "unit_price": 1000},
                             {"product_id": "b", "quantity": 1, "unit_price": 200}],
                   "subtotal": 1200, "discount": 0, "total": 1200,
                   "payments": [{"by": "p1", "amount": 500, "method": "card", "at": 1}]});
    h.append(EventKind::Placed, "r1", &r.to_string(), 100, [0; 32]).unwrap();
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    let cur = crate::room::view::current(&h, "r1").unwrap();
    let void = AmendIn {
        order_id: "r1".into(), location_id: "v1".into(), base_seq: cur.seq, ops: vec![Op::Remove { line: 1 }],
        by: "p1".into(), reason: Some("mistake".into()), may_void: false, boms: vec![], now_ms: NOW,
    };
    let (o, _, _) = decide(&mut h, &mut s, Some(&cur), &void).expect("the void lands");
    assert!(o.get("payment_status").map_or(true, Value::is_null), "{o}");
}
