use super::*;

pub(crate) fn view(id: &str, o: Value) -> OrderView {
    OrderView { order_id: id.into(), kind: 1, seq: 10, order_json: o.to_string() }
}

fn r(id: &str, sitting: &str, status: &str, total: i64, paid: i64, at: i64) -> OrderView {
    view(id, json!({"id": id, "sitting_id": sitting, "status": status, "total": total, "paid": paid,
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
