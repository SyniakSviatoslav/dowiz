//! The owner's choice for food back from a door: each refusal beside its
//! positive twin, over a real StockLog, through the real `decide`.

use super::*;
use dowiz_hub::stock::StockLevel;
use serde_json::json;

fn order(status: &str, reason: &str) -> Value {
    json!({
        "id": "o1", "status": status, "location_id": "v1", "courier_id": "courier-7",
        "total": 1200, "payment": "cash",
        "refund": {"reason": reason, "by": "courier-7", "at": 5, "from": "IN_DELIVERY"}
    })
}

fn view(o: &Value) -> OrderView {
    OrderView { order_id: "o1".into(), kind: 1, seq: 9, order_json: o.to_string() }
}

fn input(choice: &str) -> ReturnedIn {
    ReturnedIn { order_id: "o1".into(), location_id: "v1".into(), by: "owner1".into(), choice: choice.into(), now_ms: 10 }
}

/// Rice 1000, nori 10; o1 cooked (200 rice, 2 nori consumed at PREPARING).
fn cooked() -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append_all(&[
        StockEvent::Received { item: "rice".into(), qty: 1000 },
        StockEvent::Received { item: "nori".into(), qty: 10 },
        StockEvent::Reserved { item: "rice".into(), qty: 200, order_id: "o1".into() },
        StockEvent::Reserved { item: "nori".into(), qty: 2, order_id: "o1".into() },
        StockEvent::Consumed { item: "rice".into(), qty: 200, order_id: "o1".into() },
        StockEvent::Consumed { item: "nori".into(), qty: 2, order_id: "o1".into() },
    ])
    .unwrap();
    s
}

fn refused(o: &Value, s: &mut StockLog, i: &ReturnedIn) -> Refused {
    let before = s.to_bytes();
    let r = decide(s, Some(&view(o)), i).unwrap_err();
    assert_eq!(s.to_bytes(), before, "the shelf moved on a refusal");
    r
}

#[test]
fn resell_puts_every_consumed_line_back_signed_by_the_courier_and_the_chooser() {
    let mut s = cooked();
    let out = decide(&mut s, Some(&view(&order("COMPENSATED_REFUND", "refused_at_door"))), &input("resell")).expect("lands");
    assert_eq!(out.lines, vec![("rice".to_string(), 200), ("nori".to_string(), 2)]);
    assert_eq!(out.courier, "courier-7");
    let led = s.ledger().unwrap();
    assert_eq!(led.level("rice"), StockLevel { on_hand: 1000, reserved: 0 });
    assert_eq!(led.level("nori"), StockLevel { on_hand: 10, reserved: 0 });
    let last = s.events().pop().unwrap();
    assert!(matches!(last, StockEvent::Returned { ref by, ref chosen_by, resell: true, .. } if by == "courier-7" && chosen_by == "owner1"));
}

#[test]
fn waste_records_the_lines_and_leaves_the_shelf_where_consumed_put_it() {
    let mut s = cooked();
    decide(&mut s, Some(&view(&order("REFUNDING", "refused_at_door"))), &input("waste")).expect("lands");
    assert_eq!(s.ledger().unwrap().level("rice").on_hand, 800);
    assert!(returned_lines(&s, "o1").0);
}

#[test]
fn a_second_choice_is_refused() {
    let mut s = cooked();
    let o = order("COMPENSATED_REFUND", "refused_at_door");
    decide(&mut s, Some(&view(&o)), &input("waste")).expect("first lands");
    assert!(matches!(refused(&o, &mut s, &input("resell")), Refused::Conflict(_)));
    assert!(matches!(refused(&o, &mut s, &input("waste")), Refused::Conflict(_)));
}

#[test]
fn only_a_refund_at_the_door_brings_food_back() {
    let mut s = cooked();
    for (st, why) in [("COMPENSATED_REFUND", "venue_cancelled"), ("IN_DELIVERY", "refused_at_door"), ("DELIVERED", "refused_at_door")] {
        assert!(matches!(refused(&order(st, why), &mut s, &input("waste")), Refused::Conflict(_)), "{st} {why}");
    }
    // Twin: the same shelf, at the door, lands.
    assert!(decide(&mut s, Some(&view(&order("REFUNDING", "refused_at_door"))), &input("waste")).is_ok());
}

#[test]
fn an_uncooked_order_has_nothing_to_return() {
    // Released at the refund, never consumed: no lines.
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append(&StockEvent::Received { item: "rice".into(), qty: 1000 }).unwrap();
    assert!(matches!(refused(&order("COMPENSATED_REFUND", "refused_at_door"), &mut s, &input("waste")), Refused::Conflict(_)));
}

#[test]
fn the_choice_is_a_word_with_a_signer_in_this_venue() {
    let o = order("COMPENSATED_REFUND", "refused_at_door");
    let mut s = cooked();
    assert!(matches!(refused(&o, &mut s, &input("keep")), Refused::Invalid(_)));
    assert!(matches!(refused(&o, &mut s, &ReturnedIn { by: " ".into(), ..input("waste") }), Refused::Invalid(_)));
    assert!(matches!(refused(&o, &mut s, &ReturnedIn { location_id: "v2".into(), ..input("waste") }), Refused::NotFound));
    let before = s.to_bytes();
    assert!(matches!(decide(&mut s, None, &input("waste")), Err(Refused::NotFound)));
    assert_eq!(s.to_bytes(), before);
    assert!(decide(&mut s, Some(&view(&o)), &input(" resell ")).is_ok(), "twin: a valid word, trimmed");
}
