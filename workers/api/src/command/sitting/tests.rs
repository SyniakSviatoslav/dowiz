use super::*;

pub(crate) fn view(id: &str, o: Value) -> OrderView {
    OrderView { order_id: id.into(), kind: 1, seq: 10, order_json: o.to_string() }
}

/// A round with `paid` taken as ONE recorded payment — the shape `command::pay`
/// writes. This helper used to set an order-level `"paid"` field, which
/// nothing in the product ever writes: every test here passed while the room
/// read 0 paid on every live sitting.
fn r(id: &str, sitting: &str, status: &str, total: i64, paid: i64, at: i64) -> OrderView {
    let payments: Vec<Value> = if paid > 0 { vec![json!({"amount": paid, "method": "card", "at": at})] } else { vec![] };
    view(id, json!({"id": id, "sitting_id": sitting, "status": status, "total": total, "payments": payments,
                   "created_at_ms": at, "fulfilment": {"kind": "dine_in", "table": format!("t-{at}")}}))
}

/// The bill counts the rounds that took money; a rejected round is not billed.
#[test]
fn the_bill_is_the_rounds_that_took_money() {
    let listed = vec![
        r("a", "s1", "PREPARING", 1500, 0, 1),
        r("b", "s1", "REJECTED", 900, 0, 2),
        r("c", "s1", "PENDING", 400, 0, 3),
        r("x", "s2", "PENDING", 777, 0, 1),
    ];
    let rs = rounds(&listed, "s1");
    assert_eq!(rs.len(), 3);
    assert_eq!(bill(&rs), 1900);
    assert_eq!(paid(&rs), 0);
}

/// A sitting whose rounds are all over and whose bill is paid has left the
/// room; one with a cooking round or an unpaid bill has not.
#[test]
fn a_sitting_is_open_until_it_is_served_and_paid() {
    let paid_up = vec![r("a", "s1", "PICKED_UP", 1500, 1500, 1)];
    assert!(!open(&rounds(&paid_up, "s1")));
    let unpaid = vec![r("a", "s1", "PICKED_UP", 1500, 1000, 1)];
    assert!(open(&rounds(&unpaid, "s1")));
    let cooking = vec![r("a", "s1", "PREPARING", 1500, 1500, 1)];
    assert!(open(&rounds(&cooking, "s1")));
}

/// The room lists open sittings once each, at their newest round's table.
#[test]
fn the_room_is_one_card_per_open_sitting() {
    let listed = vec![
        r("a", "s1", "PREPARING", 1500, 0, 1),
        r("b", "s1", "PENDING", 400, 0, 5),
        r("c", "s2", "PICKED_UP", 900, 900, 1),
        view("d", json!({"id": "d", "status": "PENDING", "total": 100})),
    ];
    let cards = room(&listed);
    assert_eq!(cards.len(), 1);
    assert_eq!(cards[0]["sitting_id"], json!("s1"));
    assert_eq!(cards[0]["due"], json!(1900));
    assert_eq!(cards[0]["table"], json!("t-5"));
    assert_eq!(cards[0]["rounds"][0]["id"], json!("a"), "oldest round first");
}

/// PAID IS WHAT THE PAYMENTS SAY. Two payments across two rounds, one of them
/// in euro (settled at its `amount_in_order_currency`), close the sitting and
/// the card shows them; the same sitting one payment short stays open.
#[test]
fn a_sitting_paid_in_full_through_its_payments_leaves_the_room() {
    let round = |id: &str, total: i64, pays: Value| {
        view(id, json!({"id": id, "sitting_id": "s1", "status": "PICKED_UP", "total": total,
                        "payments": pays, "created_at_ms": 1}))
    };
    let full = vec![
        round("a", 1500, json!([{"amount": 1000, "method": "cash"}, {"amount": 5, "currency": "EUR", "amount_in_order_currency": 500, "method": "cash"}])),
        round("b", 400, json!([{"amount": 400, "method": "card"}])),
    ];
    let rs = rounds(&full, "s1");
    assert_eq!(paid(&rs), 1900);
    assert!(!open(&rs), "paid in full and served: the table is free");
    assert_eq!(card("s1", &rs)["rounds"][0]["paid"], json!(1500));
    assert_eq!(card("s1", &rs)["due"], json!(0));
    let short = vec![round("a", 1500, json!([{"amount": 1000, "method": "cash"}])), round("b", 400, json!([]))];
    let rs = rounds(&short, "s1");
    assert_eq!(paid(&rs), 1000);
    assert!(open(&rs), "900 still due keeps the table");
}
