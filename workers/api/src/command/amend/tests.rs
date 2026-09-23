//! G3 (BLUEPRINT-POS-THE-ROOM §6): the amendment's refusals, each beside the
//! positive twin that proves the refusal is not the only thing it can do.

use super::*;
use dowiz_hub::stock::{StockEvent, StockLog};

const NOW: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_789_999_000_000;

fn dish(pid: &str, supply: &str, qty: i64) -> (String, String) {
    (pid.into(), json!({ "id": pid, "bom": [{ "supply": supply, "qty": qty }] }).to_string())
}

fn round(status: &str) -> Value {
    json!({
        "id": "r1", "status": status, "location_id": "v1",
        "items": [
            {"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"},
            {"product_id": "beer", "quantity": 1, "unit_price": 300, "name": "Beer"}
        ],
        "subtotal": 1500, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1500,
        "fulfilment": {"kind": "dine_in", "table": "7"}, "sitting_id": "sit-000001"
    })
}

fn view(order: &Value) -> OrderView {
    OrderView { order_id: "r1".into(), kind: 1, seq: SEQ, order_json: order.to_string() }
}

fn input(ops: Vec<Op>) -> AmendIn {
    AmendIn {
        order_id: "r1".into(), location_id: "v1".into(), base_seq: SEQ, ops,
        by: "p1".into(), reason: None, may_void: false,
        boms: vec![dish("maki", "rice", 100), dish("beer", "keg", 1)], now_ms: NOW,
    }
}

fn added(pid: &str, qty: i64, unit: i64) -> Op {
    Op::Add { line: json!({"product_id": pid, "quantity": qty, "unit_price": unit, "name": pid}) }
}

/// A shelf with rice and beer, and r1's reservation on it, as placement left it.
fn shelf() -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append_all(&[
        StockEvent::Received { item: "rice".into(), qty: 1000 },
        StockEvent::Received { item: "keg".into(), qty: 10 },
    ]).unwrap();
    let lines: Vec<(String, i64)> = vec![(dish("maki", "rice", 100).1, 2), (dish("beer", "keg", 1).1, 1)];
    s.append_all(&dowiz_hub::stock::reservations_for("r1", &lines)).unwrap();
    s
}

fn hub() -> dowiz_hub::Hub {
    dowiz_hub::Hub::create_sized(64 * 1024).unwrap()
}

/// THE POSITIVE TWIN: an add lands, the money moves with the lines, the shelf
/// follows (and holds exactly the new set), and the signer is on the record.
#[test]
fn a_legal_amendment_moves_lines_money_and_shelf_together() {
    let (mut h, mut s) = (hub(), shelf());
    let (r, body, seq) = decide(&mut h, &mut s, Some(&view(&round("PENDING"))), &input(vec![added("maki", 1, 600)])).expect("lands");
    assert_eq!(r["subtotal"], json!(2100));
    assert_eq!(r["total"], json!(2100));
    assert_eq!(r["amended"][0]["by"], json!("p1"));
    assert!(seq > SEQ, "the version moves");
    assert!(body.contains("\"_d\":true"), "a delta, not a snapshot");
    assert_eq!(h.events()[0].kind, dowiz_hub::EventKind::Amended);
    let led = s.ledger().unwrap();
    assert_eq!(led.level("rice").reserved, 300, "three maki held, not two and not five");
    assert_eq!(led.level("keg").reserved, 1);
}

/// G3: no signer → refused, and NOTHING is written to either image.
#[test]
fn an_amendment_with_no_signer_is_refused_and_writes_nothing() {
    let (mut h, mut s) = (hub(), shelf());
    let (hb, sb) = (h.to_bytes(), s.to_bytes());
    let mut i = input(vec![added("maki", 1, 600)]);
    i.by = "  ".into();
    assert!(matches!(decide(&mut h, &mut s, Some(&view(&round("PENDING"))), &i), Err(Refused::Invalid(_))));
    assert_eq!(h.to_bytes(), hb);
    assert_eq!(s.to_bytes(), sb);
}

/// G3: a stale version is a Conflict for an edit to an existing line …
#[test]
fn a_stale_edit_is_refused_as_a_conflict() {
    let mut i = input(vec![Op::SetQty { line: 0, qty: 1 }]);
    i.base_seq = SEQ - 5;
    assert_eq!(
        apply(&view(&round("PENDING")), &i).unwrap_err(),
        Refused::Conflict("this order changed while you were editing it".into())
    );
}

/// … and NOT for an add, which commutes with whatever landed in between.
#[test]
fn two_adds_commute_even_from_a_stale_copy() {
    let mut i = input(vec![added("beer", 1, 300)]);
    i.base_seq = SEQ - 5;
    let (r, _) = apply(&view(&round("CONFIRMED")), &i).expect("an add is an intent, not a state");
    assert_eq!(r["items"].as_array().unwrap().len(), 3);
}

/// G3: at PREPARING without `void`, refused; with it and a reason, the line
/// goes and THE SHELF IS NOT TOUCHED — the rice was consumed at PREPARING.
#[test]
fn a_void_after_the_kitchen_needs_the_capability_and_leaves_the_shelf() {
    let mut i = input(vec![Op::Remove { line: 1 }]);
    i.reason = Some("dropped".into());
    assert_eq!(
        apply(&view(&round("PREPARING")), &i).unwrap_err(),
        Refused::Conflict("this needs the void capability".into())
    );
    i.may_void = true;
    let (mut h, mut s) = (hub(), shelf());
    let before = s.to_bytes();
    let (r, _, _) = decide(&mut h, &mut s, Some(&view(&round("PREPARING"))), &i).expect("voided");
    assert_eq!(r["total"], json!(1200));
    assert_eq!(r["amended"][0]["reason"], json!("dropped"));
    assert_eq!(s.to_bytes(), before, "no stock event after the kitchen");
}

/// G3: nothing is ADDED to what the kitchen has — that is a new round.
#[test]
fn nothing_is_added_to_a_round_the_kitchen_has() {
    let mut i = input(vec![added("maki", 1, 600)]);
    i.may_void = true;
    assert!(matches!(apply(&view(&round("READY")), &i), Err(Refused::Conflict(_))));
}

/// G3: a paid round is refused regardless of capabilities.
#[test]
fn a_paid_round_is_never_amended() {
    let mut paid = round("CONFIRMED");
    paid["payment_status"] = json!("paid");
    let mut i = input(vec![Op::Remove { line: 1 }]);
    i.reason = Some("mistake".into());
    i.may_void = true;
    assert!(matches!(apply(&view(&paid), &i), Err(Refused::Conflict(m)) if m.contains("refund")));
}

/// A line taken off says why, from the closed set.
#[test]
fn a_void_names_a_reason_from_the_closed_set() {
    let i = input(vec![Op::Remove { line: 1 }]);
    assert!(matches!(apply(&view(&round("PENDING")), &i), Err(Refused::Invalid(_))));
    let mut j = input(vec![Op::Remove { line: 1 }]);
    j.reason = Some("because".into());
    assert!(matches!(apply(&view(&round("PENDING")), &j), Err(Refused::Invalid(_))));
    j.reason = Some("other:guest found a hair".into());
    let (r, _) = apply(&view(&round("PENDING")), &j).expect("other with text is a reason");
    assert_eq!(r["amended"][0]["reason"], json!("other:guest found a hair"));
    assert_eq!(VoidReason::parse("other:"), None);
}

/// ITEM 5: a comp keeps the line, grows the discount by its amount, and law 3
/// (total = lines + fee + tip − discount) still holds.
#[test]
fn a_comp_keeps_the_line_and_moves_the_discount() {
    let mut i = input(vec![Op::Comp { line: 1 }]);
    i.reason = Some("guest_changed".into());
    assert!(matches!(apply(&view(&round("READY")), &i), Err(Refused::Conflict(_))), "a comp needs void");
    i.may_void = true;
    let (r, _) = apply(&view(&round("READY")), &i).expect("comped");
    assert_eq!(r["items"].as_array().unwrap().len(), 2, "the beer is still on the round");
    assert_eq!(r["discount"], json!(300));
    assert_eq!(r["subtotal"], json!(1500));
    assert_eq!(r["total"], json!(1200));
    assert_eq!(r["adjustments"][0]["kind"], json!("comp"));
    assert_eq!(r["adjustments"][0]["by"], json!("p1"));
    let lines = 1500; // Σ unit_price × quantity
    assert_eq!(r["total"].as_i64().unwrap(), lines + 0 + 0 - r["discount"].as_i64().unwrap());
}

/// Another venue's round is not found, as every command answers it.
#[test]
fn another_venues_round_is_not_found() {
    let mut i = input(vec![added("maki", 1, 600)]);
    i.location_id = "v2".into();
    assert_eq!(apply(&view(&round("PENDING")), &i).unwrap_err(), Refused::NotFound);
}

/// Removing every line is a cancellation, not an amendment.
#[test]
fn a_round_cannot_be_amended_to_nothing() {
    let mut i = input(vec![Op::Remove { line: 0 }, Op::Remove { line: 1 }]);
    i.reason = Some("guest_changed".into());
    assert!(matches!(apply(&view(&round("PENDING")), &i), Err(Refused::Conflict(_))));
}

/// Moving a round to another table is allowed while it cooks.
#[test]
fn a_round_moves_table_while_it_cooks() {
    let i = input(vec![Op::Table { table: "12".into() }]);
    let (r, restock) = apply(&view(&round("PREPARING")), &i).expect("moved");
    assert_eq!(r["fulfilment"]["table"], json!("12"));
    assert_eq!(r["total"], json!(1500), "a move is not a price change");
    assert!(!restock);
}
