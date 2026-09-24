//! A10: the refund's refusals, each beside its positive twin, all through
//! `decide` over a real Hub and a real StockLog.

use super::*;
use dowiz_hub::stock::{StockEvent, StockLog};

const NOW: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_789_999_000_000;

fn order(status: &str) -> Value {
    json!({
        "id": "o1", "status": status, "location_id": "v1", "created_at_ms": 1,
        "items": [{"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"}],
        "subtotal": 1200, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1200,
        "payment": "cash", "at": {"CONFIRMED": 5}
    })
}

fn view(o: &Value) -> OrderView {
    OrderView { order_id: "o1".into(), kind: 1, seq: SEQ, order_json: o.to_string() }
}

fn input(reason: &str) -> RefundIn {
    RefundIn {
        order_id: "o1".into(), location_id: "v1".into(), by: "p1".into(),
        reason: reason.into(), complete: false, now_ms: NOW, at_door: false, note: None,
    }
}

/// A shelf with rice, and o1's reservation on it, as placement left it.
fn shelf() -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append_all(&[StockEvent::Received { item: "rice".into(), qty: 1000 }]).unwrap();
    let maki = json!({ "id": "maki", "bom": [{ "supply": "rice", "qty": 100 }] }).to_string();
    s.append_all(&dowiz_hub::stock::reservations_for("o1", &[(maki, 2)])).unwrap();
    s
}

/// A hub holding o1 as placed, so the fold of what the refund wrote can be read.
fn hub_with(o: &Value) -> dowiz_hub::Hub {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    h.append(EventKind::Placed, "o1", &o.to_string(), SEQ, [0u8; 32]).unwrap();
    h
}

fn folded(h: &dowiz_hub::Hub) -> Value {
    let listed = crate::hubstore::orders_state(h);
    serde_json::from_str(&listed[0].order_json).unwrap()
}

/// Every refusal: nothing is written to either image.
fn refused(o: &Value, i: &RefundIn) -> Refused {
    let (mut h, mut s) = (hub_with(o), shelf());
    let (hb, sb) = (h.to_bytes(), s.to_bytes());
    let r = decide(&mut h, &mut s, Some(&view(o)), i, "ALL").unwrap_err();
    assert_eq!(h.to_bytes(), hb, "the log moved on a refusal");
    assert_eq!(s.to_bytes(), sb, "the shelf moved on a refusal");
    r
}

/// THE POSITIVE TWIN of all of them, and the cancellation case: CONFIRMED, no
/// money taken → REFUNDING → COMPENSATED_REFUND in one turn, the hold
/// released, the signer on the record, law 3 untouched, seqs strictly rising.
#[test]
fn a_confirmed_order_with_no_money_ends_compensated_and_releases_its_hold() {
    let o = order("CONFIRMED");
    let (mut h, mut s) = (hub_with(&o), shelf());
    assert_eq!(s.ledger().unwrap().level("rice").reserved, 200);
    let (m, w) = decide(&mut h, &mut s, Some(&view(&o)), &input("venue_cancelled"), "ALL").expect("lands");
    let kinds: Vec<EventKind> = w.iter().map(|e| e.0).collect();
    assert_eq!(kinds, vec![EventKind::Advanced, EventKind::Noted, EventKind::Advanced]);
    assert!(w[0].2 > SEQ && w[1].2 > w[0].2 && w[2].2 > w[1].2, "seqs strictly increase: {w:?}");
    assert_eq!(m["status"], json!("COMPENSATED_REFUND"));
    assert_eq!(m["refund"]["by"], json!("p1"));
    assert_eq!(m["refund"]["reason"], json!("venue_cancelled"));
    assert_eq!((m["refund"]["owed"].clone(), m["refund"]["money_taken"].clone()), (json!(0), json!(false)));
    assert!(w[1].1.contains("\"by\":\"p1\""), "the signer is in the Noted body");
    let led = s.ledger().unwrap();
    assert_eq!(led.level("rice").reserved, 0, "never cooked: released");
    assert!(led.stranded().is_empty());
    // THE FOLD agrees with what decide returned, and kept the whole order.
    let f = folded(&h);
    assert_eq!(f, m);
    for k in ["items", "subtotal", "discount", "tip", "total", "location_id"] {
        assert_eq!(f[k], o[k], "{k} moved");
    }
}

/// PREPARING: the ingredients were consumed there, so the shelf is untouched.
#[test]
fn a_refund_after_the_kitchen_took_it_leaves_the_shelf() {
    let o = order("PREPARING");
    let (mut h, mut s) = (hub_with(&o), shelf());
    let before = s.to_bytes();
    let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &input("customer_request"), "ALL").expect("lands");
    assert_eq!(m["status"], json!("COMPENSATED_REFUND"));
    assert_eq!(s.to_bytes(), before, "the kitchen had it: nothing released");
}

/// Refused at the door with cash in the courier's hand: the amount is
/// recorded and the order WAITS in REFUNDING until the money is handed back;
/// `complete` is the second edge, and it too is signed.
#[test]
fn refused_at_door_with_cash_records_the_amount_and_waits_for_the_handback() {
    let mut o = order("IN_DELIVERY");
    o["cash_collected"] = json!(1200);
    o["at"]["PREPARING"] = json!(6);
    let (mut h, mut s) = (hub_with(&o), shelf());
    let shelf_before = s.to_bytes();
    let (m, w) = decide(&mut h, &mut s, Some(&view(&o)), &input("refused_at_door"), "ALL").expect("lands");
    assert_eq!(s.to_bytes(), shelf_before, "cooked at PREPARING: the shelf is the owner's call");
    assert_eq!(m["status"], json!("REFUNDING"));
    assert_eq!(w.len(), 2);
    assert_eq!((m["refund"]["owed"].clone(), m["refund"]["money_taken"].clone()), (json!(1200), json!(true)));
    assert_eq!(m["refund"]["from"], json!("IN_DELIVERY"));
    assert_eq!(m["total"], json!(1200), "law 3");

    let mut c = input("");
    c.complete = true;
    c.by = "p2".into();
    let v = OrderView { order_id: "o1".into(), kind: 1, seq: w[1].2, order_json: m.to_string() };
    let (done, w2) = decide(&mut h, &mut s, Some(&v), &c, "ALL").expect("completes");
    assert_eq!(done["status"], json!("COMPENSATED_REFUND"));
    assert_eq!(done["refund"]["returned"]["by"], json!("p2"));
    assert_eq!(done["refund"]["owed"], json!(1200), "the first record survives");
    assert!(w2[0].2 > w[1].2 && w2[1].2 > w2[0].2);
    assert_eq!(folded(&h), done);
}

/// CONFIRMED → IN_DELIVERY (a bottle, never cooked): the hold is released
/// even though the order is already on the road — the status alone is not
/// the question, `at.PREPARING` is.
#[test]
fn an_uncooked_order_on_the_road_releases_its_hold() {
    let o = order("IN_DELIVERY");
    let (mut h, mut s) = (hub_with(&o), shelf());
    let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &input("refused_at_door"), "ALL").expect("lands");
    assert_eq!(m["status"], json!("COMPENSATED_REFUND"), "cash order, nothing collected: nothing owed");
    let led = s.ledger().unwrap();
    assert_eq!(led.level("rice").reserved, 0);
    assert!(led.stranded().is_empty());
}

/// READY (the blueprint's CHECK): the refund lands and nothing is left held.
#[test]
fn a_ready_order_refunds_and_nothing_is_stranded() {
    let mut o = order("READY");
    o["at"]["PREPARING"] = json!(6);
    let mut s = shelf();
    let led = s.ledger().unwrap();
    s.append_all(&dowiz_hub::stock::settle(&led, "o1", true)).unwrap();
    let mut h = hub_with(&o);
    let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &input("customer_request"), "ALL").expect("lands");
    assert_eq!(m["status"], json!("COMPENSATED_REFUND"));
    assert!(s.ledger().unwrap().stranded().is_empty());
}

/// A CLOCK BEHIND THE LOG: every seq is still past the order's own, and rising.
#[test]
fn seqs_rise_past_the_order_even_when_the_clock_is_behind() {
    let o = order("CONFIRMED");
    let (mut h, mut s) = (hub_with(&o), shelf());
    let mut i = input("venue_cancelled");
    i.now_ms = 5;
    let (_, w) = decide(&mut h, &mut s, Some(&view(&o)), &i, "ALL").expect("lands");
    assert_eq!(w.iter().map(|e| e.2).collect::<Vec<_>>(), vec![SEQ + 1, SEQ + 2, SEQ + 3]);
}

/// Money from the room's payments and from the card webhook both count.
#[test]
fn owed_is_every_payment_in_the_order_currency() {
    let mut o = order("READY");
    o["payments"] = json!([{"amount": 500}, {"amount": 10, "amount_in_order_currency": 700}]);
    assert_eq!(money_taken(&o).unwrap(), (1200, true));
    let mut card = order("READY");
    card["payment_status"] = json!("paid");
    card["amount_received"] = json!(1150);
    assert_eq!(money_taken(&card).unwrap(), (1150, true));
    card.as_object_mut().unwrap().remove("amount_received");
    assert_eq!(money_taken(&card).unwrap(), (1200, true), "paid with no record: its total");
    assert_eq!(money_taken(&order("READY")).unwrap(), (0, false));
}

#[test]
fn no_signer_is_refused() {
    let mut i = input("venue_cancelled");
    i.by = "  ".into();
    assert!(matches!(refused(&order("CONFIRMED"), &i), Refused::Invalid(_)));
}

#[test]
fn a_reason_outside_the_set_is_refused() {
    for bad in ["soggy", "", "other:", "other: ", "REFUSED_AT_DOOR"] {
        assert!(matches!(refused(&order("CONFIRMED"), &input(bad)), Refused::Invalid(_)), "{bad:?}");
    }
    assert_eq!(RefundReason::parse("other:guest fell ill"), Some(RefundReason::Other("guest fell ill".into())));
    for w in ["refused_at_door", "venue_cancelled", "customer_request", "payment_error"] {
        assert_eq!(RefundReason::parse(w).unwrap().word(), w);
    }
}

/// Terminal orders have no refund edge in the kernel; PENDING is cancelled.
#[test]
fn terminal_and_pending_orders_are_refused_as_conflicts() {
    for st in ["DELIVERED", "PICKED_UP", "CANCELLED", "REJECTED", "COMPENSATED_REFUND", "REFUNDING", "PENDING"] {
        assert!(matches!(refused(&order(st), &input("venue_cancelled")), Refused::Conflict(_)), "{st}");
    }
    let mut c = input("");
    c.complete = true;
    assert!(matches!(refused(&order("CONFIRMED"), &c), Refused::Conflict(_)), "complete needs REFUNDING");
}

#[test]
fn another_venues_order_is_not_found() {
    let mut i = input("venue_cancelled");
    i.location_id = "v2".into();
    assert_eq!(refused(&order("CONFIRMED"), &i), Refused::NotFound);
    let (mut h, mut s) = (hub_with(&order("CONFIRMED")), shelf());
    assert_eq!(decide(&mut h, &mut s, None, &input("venue_cancelled"), "ALL").unwrap_err(), Refused::NotFound);
}

// ── the courier's tap: `at_door` (§2.4) ──

fn at_door() -> RefundIn {
    RefundIn { by: "courier-7".into(), at_door: true, ..input("refused_at_door") }
}

#[test]
fn the_couriers_tap_ends_a_cash_run_with_zero_collected() {
    let o = order("IN_DELIVERY");
    let (mut h, mut s) = (hub_with(&o), shelf());
    let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &at_door(), "ALL").expect("lands");
    assert_eq!(m["status"], json!("COMPENSATED_REFUND"), "nothing taken: nothing owed");
    assert_eq!(m["cash_collected"], json!(0));
    assert_eq!((m["refund"]["by"].clone(), m["refund"]["reason"].clone()), (json!("courier-7"), json!("refused_at_door")));
    assert_eq!(folded(&h), m, "the log says what the answer says");
    // Twin: the owner's refund of the same order records no cash of its own.
    let (mut h2, mut s2) = (hub_with(&o), shelf());
    let (m2, _) = decide(&mut h2, &mut s2, Some(&view(&o)), &input("refused_at_door"), "ALL").expect("lands");
    assert!(m2.get("cash_collected").is_none());
}

#[test]
fn the_couriers_tap_on_a_paid_card_order_waits_for_the_handback() {
    let mut o = order("IN_DELIVERY");
    o["payment"] = json!("card");
    o["amount_received"] = json!(1200);
    let (mut h, mut s) = (hub_with(&o), shelf());
    let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &at_door(), "ALL").expect("lands");
    assert_eq!(m["status"], json!("REFUNDING"));
    assert_eq!((m["refund"]["owed"].clone(), m["cash_collected"].clone()), (json!(1200), json!(0)));
}

#[test]
fn the_couriers_tap_is_only_at_the_door() {
    // READY / CONFIRMED: the food never left, so it was not refused at a door.
    for st in ["READY", "CONFIRMED"] {
        assert!(matches!(refused(&order(st), &at_door()), Refused::Conflict(_)), "{st}");
    }
    // Twin: the owner may still refund a READY order.
    let o = order("READY");
    let (mut h, mut s) = (hub_with(&o), shelf());
    assert!(decide(&mut h, &mut s, Some(&view(&o)), &input("venue_cancelled"), "ALL").is_ok());
}

#[test]
fn the_couriers_tap_has_one_reason_and_cannot_complete() {
    let o = order("IN_DELIVERY");
    let other = RefundIn { reason: "customer_request".into(), ..at_door() };
    assert!(matches!(refused(&o, &other), Refused::Invalid(_)));
    let done = RefundIn { complete: true, ..at_door() };
    assert!(matches!(refused(&o, &done), Refused::Invalid(_)));
}

// ── the note (optional, bounded) ──

#[test]
fn a_note_is_carried_into_the_refund_record() {
    let o = order("IN_DELIVERY");
    let (mut h, mut s) = (hub_with(&o), shelf());
    let i = RefundIn { note: Some("  nobody opened  ".into()), ..at_door() };
    let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &i, "ALL").expect("lands");
    assert_eq!(m["refund"]["note"], json!("nobody opened"));
    assert_eq!(folded(&h), m);
    // Twin: no note, or a blank one, writes no key.
    for n in [None, Some("   ".to_string())] {
        let (mut h, mut s) = (hub_with(&o), shelf());
        let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &RefundIn { note: n, ..at_door() }, "ALL").expect("lands");
        assert!(m["refund"].get("note").is_none());
    }
}

#[test]
fn a_note_past_the_bound_is_refused_and_one_at_it_lands() {
    let o = order("IN_DELIVERY");
    let long = RefundIn { note: Some("x".repeat(NOTE_MAX_CHARS + 1)), ..at_door() };
    assert!(matches!(refused(&o, &long), Refused::Invalid(_)));
    let (mut h, mut s) = (hub_with(&o), shelf());
    let edge = RefundIn { note: Some("\u{0161}".repeat(NOTE_MAX_CHARS)), ..at_door() };
    assert!(decide(&mut h, &mut s, Some(&view(&o)), &edge, "ALL").is_ok(), "characters, not bytes");
}

// ── the currency the refund is recorded in ──

/// A ROUND AS THE ROOM PLACES IT (no top-level `currency`), paid by the REAL
/// `pay::decide` on the same log, then refunded: the record names the venue's
/// currency, never null, and the exceptions view groups it there, not under "".
#[test]
fn a_paid_round_with_no_currency_is_refunded_in_the_venues() {
    use crate::command::pay::{decide as pay, PayIn, Room};
    let placed = order("CONFIRMED");
    assert!(placed.get("currency").is_none(), "the room writes no currency on a round");
    let (mut h, mut s) = (hub_with(&placed), shelf());
    let take = PayIn {
        order_id: "o1".into(), location_id: "v1".into(), amount: 1200, method: "card".into(), by: "p1".into(),
        till_id: None, covers: None, currency: None, rate_ppm: None, tip: Some(100), wallet: None, now_ms: NOW - 10,
    };
    let (paid, _, pseq) = pay(&mut h, Some(&view(&placed)), &take, &Room { open_till: None, venue_currency: "ALL" }).expect("paid");
    let v = OrderView { order_id: "o1".into(), kind: 3, seq: pseq, order_json: paid.to_string() };
    let (m, _) = decide(&mut h, &mut s, Some(&v), &input("customer_request"), "ALL").expect("lands");
    assert_eq!(m["refund"]["currency"], json!("ALL"), "never null");
    assert_eq!(m["refund"]["owed"], json!(1300), "the bill and the tip");
    assert_eq!(folded(&h)["refund"]["currency"], json!("ALL"), "the log holds it, not only the answer");
    let rows = crate::exceptions::fold::order_rows(&h.events_oldest_first(), i64::MAX);
    let refunds: Vec<_> = rows.iter().filter(|r| r.kind == crate::exceptions::fold::REFUND).collect();
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0].currency.as_deref(), Some("ALL"));
}

/// Its twin: an order that names its own currency is refunded in that one,
/// whatever the venue's is.
#[test]
fn an_order_that_names_its_currency_is_refunded_in_it() {
    let mut o = order("CONFIRMED");
    o["currency"] = json!("EUR");
    let (mut h, mut s) = (hub_with(&o), shelf());
    let (m, _) = decide(&mut h, &mut s, Some(&view(&o)), &input("venue_cancelled"), "ALL").expect("lands");
    assert_eq!(m["refund"]["currency"], json!("EUR"));
    assert_eq!(currency_of(&json!({"currency": " "}), "ALL"), "ALL", "a blank name is no name");
}
