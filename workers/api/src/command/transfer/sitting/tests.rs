//! §2.9 "move a whole sitting": every round still in the room gets the new
//! table, each in its own signed delta, in one decision; the bill does not
//! change; a paid round in the room refuses the whole move; a refusal writes
//! nothing.

use super::*;
use serde_json::json;

const NOW: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_789_999_000_000;

fn round(id: &str, status: &str, created: i64) -> Value {
    json!({
        "id": id, "status": status, "location_id": "v1", "created_at_ms": created,
        "items": [{"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"}],
        "subtotal": 1200, "discount": 0, "delivery_fee": 0, "tip": 100, "total": 1300,
        "fulfilment": {"kind": "dine_in", "table": "7"}, "sitting_id": "sit-1"
    })
}

fn view(o: &Value) -> OrderView {
    OrderView { order_id: o["id"].as_str().unwrap().into(), kind: 1, seq: SEQ, order_json: o.to_string() }
}

fn input() -> MoveIn {
    MoveIn { sitting_id: "sit-1".into(), location_id: "v1".into(), table: "12".into(), by: "p1".into(), now_ms: NOW }
}

fn hub() -> dowiz_hub::Hub {
    dowiz_hub::Hub::create_sized(64 * 1024).unwrap()
}

/// Three rounds of sit-1 — one before the kitchen, one in it, one served —
/// and one round of another sitting at the same table.
fn room() -> Vec<OrderView> {
    let mut other = round("x1", "PENDING", 4);
    other["sitting_id"] = json!("sit-2");
    vec![view(&round("a", "CONFIRMED", 1)), view(&round("b", "PREPARING", 2)), view(&round("c", "DELIVERED", 3)), view(&other)]
}

fn refused(listed: &[OrderView], i: &MoveIn) -> Refused {
    let mut h = hub();
    let before = h.to_bytes();
    let r = decide(&mut h, listed, i).err().expect("it landed");
    assert_eq!(h.to_bytes(), before, "the log was written on a refusal");
    r
}

/// THE POSITIVE TWIN: both live rounds move (the one in the kitchen too), the
/// served one keeps its table, the other sitting is untouched, each moved
/// round has its own signed Amended delta, and no total changed.
#[test]
fn every_live_round_moves_and_the_bill_does_not() {
    let mut h = hub();
    let listed = room();
    let moved = decide(&mut h, &listed, &input()).expect("moves");
    let ids: Vec<&str> = moved.iter().map(|m| m.order_id.as_str()).collect();
    assert_eq!(ids, ["a", "b"], "the served round and the other sitting stay");
    for m in &moved {
        let r: Value = serde_json::from_str(&m.merged).unwrap();
        assert_eq!(r["fulfilment"]["table"], json!("12"));
        assert_eq!(r["total"], json!(1300), "the bill does not change");
        assert_eq!(r["sitting_id"], json!("sit-1"), "the sitting id does not change");
        assert_eq!(r["amended"][0]["by"], json!("p1"));
        assert!(m.body.contains("\"_d\":true") && m.body.contains("\"12\""), "a delta with the table");
        assert!(!m.body.contains("\"total\""), "no money in the delta");
        assert!(m.seq > SEQ);
    }
    let ev = h.events();
    assert_eq!(ev.len(), 2, "one Amended per moved round");
    assert!(ev.iter().all(|e| e.kind == dowiz_hub::EventKind::Amended));
}

/// A round already at the new table is not re-signed; nothing to move at all
/// is said, not written.
#[test]
fn a_round_already_there_is_skipped_and_nothing_to_move_is_refused() {
    let mut there = round("a", "CONFIRMED", 1);
    there["fulfilment"]["table"] = json!("12");
    let listed = vec![view(&there), view(&round("b", "PENDING", 2))];
    let moved = decide(&mut hub(), &listed, &input()).expect("moves b");
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].order_id, "b");
    assert!(matches!(refused(&[view(&there)], &input()), Refused::Invalid(_)));
}

/// THE CONFIRMED FRAME: a paid round still in the room refuses the whole move
/// and names itself; a paid round that was SERVED is history and does not.
#[test]
fn a_paid_live_round_refuses_the_whole_move() {
    let mut listed = room();
    let mut paid = round("b", "READY", 2);
    paid["payment_status"] = json!("paid");
    listed[1] = view(&paid);
    assert!(matches!(refused(&listed, &input()), Refused::Conflict(m) if m.contains("round b is paid")));
    // Twin: the served round `c` is paid too, and the move lands.
    let mut listed = room();
    let mut served = round("c", "DELIVERED", 3);
    served["payment_status"] = json!("paid");
    listed[2] = view(&served);
    assert_eq!(decide(&mut hub(), &listed, &input()).expect("moves").len(), 2);
}

/// A round whose stored total does not add up is not silently re-summed by a
/// table change: the move is refused, and it names the round.
#[test]
fn a_round_that_does_not_add_up_is_not_repriced_by_a_move() {
    let mut bad = round("a", "PENDING", 1);
    bad["total"] = json!(999);
    assert!(matches!(refused(&[view(&bad)], &input()), Refused::Conflict(m) if m.contains("round a")));
}

/// No signer, no table, a sitting nobody has, another venue's sitting.
#[test]
fn the_signer_the_table_and_the_venue_are_checked() {
    let mut i = input();
    i.by = "".into();
    assert!(matches!(refused(&room(), &i), Refused::Invalid(_)));
    let mut i = input();
    i.table = "  ".into();
    assert!(matches!(refused(&room(), &i), Refused::Invalid(_)));
    let mut i = input();
    i.sitting_id = "nobody".into();
    assert_eq!(refused(&room(), &i), Refused::NotFound);
    let mut i = input();
    i.location_id = "v2".into();
    assert_eq!(refused(&room(), &i), Refused::NotFound);
}

/// A ROUND WITH SOME MONEY ON IT BUT NOT SETTLED still moves: the confirmed
/// frame is `payment_status: paid`, and a table is not money. And the log
/// folds: each moved round's `Placed` + `Amended` rebuilds what was decided.
#[test]
fn a_partly_paid_round_moves_and_the_log_folds_to_it() {
    let mut part = round("a", "CONFIRMED", 1);
    part["payments"] = json!([{"amount": 700, "method": "card"}]);
    let listed = vec![view(&part)];
    let mut h = hub();
    h.append(dowiz_hub::EventKind::Placed, "a", &part.to_string(), SEQ, [0u8; 32]).unwrap();
    let moved = decide(&mut h, &listed, &input()).expect("moves");
    let folded = crate::fold::fold(h.history("a").iter().map(|e| e.order_json.as_str()));
    let want: Value = serde_json::from_str(&moved[0].merged).unwrap();
    assert_eq!(folded, want);
    assert_eq!(folded["payments"], part["payments"], "the payment stays with its round");
}
