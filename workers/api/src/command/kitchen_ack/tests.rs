//! A13 (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.6): every refusal beside the
//! positive twin that proves the refusal is not the only thing it can do.

use super::*;

const PLACED: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_790_000_000_000;

fn order(status: &str) -> Value {
    json!({
        "id": "r1", "status": status, "location_id": "v1", "created_at_ms": PLACED,
        "items": [{"product_id": "maki", "quantity": 2, "unit_price": 600}], "total": 1200
    })
}

fn view(o: &Value) -> OrderView {
    OrderView { order_id: "r1".into(), kind: 1, seq: SEQ, order_json: o.to_string() }
}

fn ack(by: &str, at: i64) -> KitchenAckIn {
    KitchenAckIn { order_id: "r1".into(), location_id: "v1".into(), by: by.into(), now_ms: at }
}

fn hub() -> dowiz_hub::Hub {
    dowiz_hub::Hub::create_sized(64 * 1024).unwrap()
}

/// A log holding r1 as placed, the way `orders_state` will fold it.
fn placed_log() -> dowiz_hub::Hub {
    let mut h = hub();
    h.append(dowiz_hub::EventKind::Placed, "r1", &order("PENDING").to_string(), SEQ, [0u8; 32]).unwrap();
    h
}

/// THE POSITIVE TWIN of every refusal: one `Noted`, the signer and instant on
/// the order, the status unmoved, the version moved.
#[test]
fn an_ack_writes_one_noted_with_who_and_when() {
    let mut h = hub();
    let (o, w) = decide(&mut h, Some(&view(&order("PENDING"))), &ack("cook1", PLACED + 5_000)).expect("lands");
    let (body, seq) = w.expect("a first ack writes");
    assert_eq!(o["kitchen"]["seen"], json!({"by": "cook1", "at": PLACED + 5_000}));
    assert_eq!(o["status"], json!("PENDING"), "a fact, not a transition");
    assert!(body.contains("\"_d\":true"), "a delta, not a snapshot");
    assert!(seq > SEQ, "the version moves");
    assert_eq!(h.events().len(), 1);
    assert_eq!(h.events()[0].kind, dowiz_hub::EventKind::Noted);
}

/// THE FOLD THE HEALTH PANE CALLS, over a real log: unacked past N is listed;
/// the same log after an ack through `decide` is not.
#[test]
fn an_unacked_ticket_past_n_is_listed_and_an_acked_one_is_not() {
    let late = PLACED + UNSEEN_AFTER_MS + 1;
    let mut h = placed_log();
    let listed = unconfirmed_in(&h, late);
    assert_eq!(listed, vec![Unseen { order_id: "r1".into(), status: "PENDING".into(), age_ms: UNSEEN_AFTER_MS + 1, printed_at: None }]);

    let current = crate::hubstore::orders_state(&h).into_iter().next().map(OrderView::of_event);
    decide(&mut h, current.as_ref(), &ack("cook1", PLACED + 60_000)).expect("lands");
    assert_eq!(unconfirmed_in(&h, late), vec![], "the Noted folds into the order the pane reads");
}

/// Before N nothing is listed; AT N it is (the boundary is inclusive).
#[test]
fn an_unacked_ticket_before_n_is_not_listed_and_at_n_it_is() {
    let o = order("CONFIRMED");
    assert!(unconfirmed([&o], PLACED + UNSEEN_AFTER_MS - 1).is_empty());
    assert_eq!(unconfirmed([&o], PLACED + UNSEEN_AFTER_MS).len(), 1);
}

/// A kitchen that has ACTED on the order saw it; a finished, scheduled or
/// unaged order owes nothing. Twin: the same order at PENDING is owed.
#[test]
fn only_owed_statuses_with_a_placement_instant_are_listed() {
    let late = PLACED + UNSEEN_AFTER_MS * 10;
    for s in ["PREPARING", "READY", "IN_DELIVERY", "DELIVERED", "CANCELLED", "SCHEDULED"] {
        assert!(unconfirmed([&order(s)], late).is_empty(), "{s} is not owed a seen");
    }
    let mut unaged = order("PENDING");
    unaged.as_object_mut().unwrap().remove("created_at_ms");
    assert!(unconfirmed([&unaged], late).is_empty(), "no instant, no guess");
    assert_eq!(unconfirmed([&order("PENDING")], late).len(), 1);
}

/// Oldest first: the ticket the kitchen has missed longest is the first row.
#[test]
fn the_oldest_unseen_ticket_comes_first() {
    let mut b = order("PENDING");
    b["id"] = json!("r2");
    b["created_at_ms"] = json!(PLACED - 60_000);
    let got = unconfirmed([&order("PENDING"), &b], PLACED + UNSEEN_AFTER_MS);
    assert_eq!(got.iter().map(|u| u.order_id.as_str()).collect::<Vec<_>>(), vec!["r2", "r1"]);
}

/// An order this venue does not have → NotFound, and nothing written.
#[test]
fn an_ack_for_an_unknown_order_is_not_found_and_writes_nothing() {
    let mut h = hub();
    let before = h.to_bytes();
    assert_eq!(decide(&mut h, None, &ack("cook1", PLACED)).unwrap_err(), Refused::NotFound);
    assert_eq!(h.to_bytes(), before, "hub bytes unchanged on refusal");
    assert!(decide(&mut h, Some(&view(&order("PENDING"))), &ack("cook1", PLACED)).is_ok(), "twin: a known order lands");
}

/// Another venue's order → NotFound (never "forbidden"), nothing written.
#[test]
fn an_ack_for_another_venues_order_is_not_found_and_writes_nothing() {
    let mut h = hub();
    let before = h.to_bytes();
    let mut i = ack("cook1", PLACED);
    i.location_id = "v2".into();
    assert_eq!(decide(&mut h, Some(&view(&order("PENDING"))), &i).unwrap_err(), Refused::NotFound);
    assert_eq!(h.to_bytes(), before, "hub bytes unchanged on refusal");
    i.location_id = "v1".into();
    assert!(decide(&mut h, Some(&view(&order("PENDING"))), &i).is_ok(), "twin: its own venue lands");
}

/// No signer → Invalid, nothing written; a named one lands.
#[test]
fn an_ack_with_no_signer_is_refused_and_writes_nothing() {
    let mut h = hub();
    let before = h.to_bytes();
    for by in ["", "   "] {
        let r = decide(&mut h, Some(&view(&order("PENDING"))), &ack(by, PLACED));
        assert!(matches!(r, Err(Refused::Invalid(_))), "{by:?} refused");
    }
    assert_eq!(h.to_bytes(), before, "hub bytes unchanged on refusal");
    assert!(decide(&mut h, Some(&view(&order("PENDING"))), &ack("cook1", PLACED)).is_ok(), "twin");
}

/// A REPEATED ACK IS ONE FACT: the second tap writes nothing and the first
/// signer and instant stand.
#[test]
fn a_repeated_ack_is_one_fact() {
    let mut h = hub();
    let (first, w) = decide(&mut h, Some(&view(&order("PENDING"))), &ack("cook1", PLACED + 1_000)).unwrap();
    assert!(w.is_some(), "the first tap writes");
    let before = h.to_bytes();
    let (again, w2) = decide(&mut h, Some(&view(&first)), &ack("cook2", PLACED + 9_000)).unwrap();
    assert!(w2.is_none(), "the second tap writes nothing");
    assert_eq!(h.to_bytes(), before);
    assert_eq!(h.events().len(), 1);
    assert_eq!(again["kitchen"]["seen"], json!({"by": "cook1", "at": PLACED + 1_000}));
}

/// PRINTED IS NOT SEEN (§2.6): a printed ticket nobody tapped is still listed,
/// and says it was printed. Twin: the same order once seen is not listed.
#[test]
fn a_printed_unseen_ticket_is_still_listed_and_says_it_printed() {
    let mut o = order("PENDING");
    o["kitchen"] = json!({"printed": {"at": PLACED + 2_000, "printer": "kitchen", "code": "200 OK"}});
    let got = unconfirmed([&o], PLACED + UNSEEN_AFTER_MS);
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].printed_at, Some(PLACED + 2_000));
    o["kitchen"]["seen"] = json!({"by": "cook1", "at": PLACED + 3_000});
    assert!(unconfirmed([&o], PLACED + UNSEEN_AFTER_MS).is_empty());
}
