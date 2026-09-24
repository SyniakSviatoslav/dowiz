//! G5 (BLUEPRINT-POS-THE-ROOM §6): the payment's refusals, each beside the
//! positive twin that proves the refusal is not the only thing it can do.
//! Plus the two rules this file gained: cash needs an open till, and a
//! payment in another currency states its rate (P3-2).

/// P1: money conservation over random histories of one round (G4, G5).
/// Test code, so it lives under `tests/` where the shipping-code gates
/// (`channel-closed`) do not count its `Placed` append as a shipping site.
mod conservation;

use super::*;

const NOW: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_789_999_000_000;

/// The till `main` is open, the venue is in lek.
const OPEN: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };
/// No till open.
const SHUT: Room<'static> = Room { open_till: None, venue_currency: "ALL" };

fn round(status: &str, total: i64) -> Value {
    json!({
        "id": "r1", "status": status, "location_id": "v1",
        "items": [{"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"}],
        "subtotal": total, "discount": 0, "delivery_fee": 0, "tip": 0, "total": total,
        "fulfilment": {"kind": "dine_in", "table": "7"}, "sitting_id": "sit-000001"
    })
}

fn view(order: &Value) -> OrderView {
    OrderView { order_id: "r1".into(), kind: 3, seq: SEQ, order_json: order.to_string() }
}

fn input(amount: i64, method: &str) -> PayIn {
    PayIn {
        order_id: "r1".into(),
        location_id: "v1".into(),
        amount,
        method: method.into(),
        by: "p1".into(),
        till_id: None,
        covers: None,
        currency: None,
        rate_ppm: None,
        tip: None,
        wallet: None,
        base_seq: None,
        now_ms: NOW,
    }
}

fn eur(amount: i64, rate: Option<i64>) -> PayIn {
    PayIn { currency: Some("EUR".into()), rate_ppm: rate, ..input(amount, "cash") }
}

fn hub() -> dowiz_hub::Hub {
    dowiz_hub::Hub::create_sized(64 * 1024).unwrap()
}

fn pay(h: &mut dowiz_hub::Hub, order: &Value, i: &PayIn, room: &Room) -> Result<(Value, String, u64), Refused> {
    decide(h, Some(&view(order)), i, room)
}

/// THE POSITIVE TWIN: 600 lands on a 1500 round, the round stays unpaid, and
/// the signer, the currency and the till are on the record.
#[test]
fn a_legal_partial_cash_payment_lands_in_the_open_till() {
    let mut h = hub();
    let (order, body, seq) = pay(&mut h, &round("PENDING", 1500), &input(600, "cash"), &OPEN).expect("lands");
    let p = &order["payments"][0];
    assert_eq!((p["amount"].clone(), p["by"].clone(), p["method"].clone()), (json!(600), json!("p1"), json!("cash")));
    assert_eq!(p["currency"], json!("ALL"), "no currency named is the order's");
    assert_eq!(p["till_id"], json!("main"), "the object stamps the open till");
    assert_eq!(p.get("rate_ppm"), None, "a same-currency payment has no rate");
    assert_eq!(order.get("payment_status"), None, "not paid yet");
    assert_eq!((order["total"].clone(), order["subtotal"].clone()), (json!(1500), json!(1500)));
    assert!(seq > SEQ, "the version moves");
    assert!(body.contains("\"_d\":true"), "a delta, not a snapshot");
    assert_eq!(h.events()[0].kind, dowiz_hub::EventKind::Paid);
}

#[test]
fn cash_with_no_till_open_is_refused_and_writes_nothing_and_card_lands() {
    let mut h = hub();
    let before = h.to_bytes();
    assert!(matches!(pay(&mut h, &round("PENDING", 1500), &input(600, "cash"), &SHUT), Err(Refused::Conflict(_))));
    assert_eq!(h.to_bytes(), before, "nothing written");
    let (order, _, _) = pay(&mut h, &round("PENDING", 1500), &input(600, "card"), &SHUT).expect("card needs no drawer");
    assert_eq!(order["payments"][0].get("till_id"), None);
}

#[test]
fn cash_naming_a_till_that_is_not_open_is_refused_and_naming_the_open_one_lands() {
    let mut h = hub();
    let wrong = PayIn { till_id: Some("bar".into()), ..input(600, "cash") };
    assert!(matches!(pay(&mut h, &round("PENDING", 1500), &wrong, &OPEN), Err(Refused::Conflict(_))));
    let right = PayIn { till_id: Some("main".into()), ..input(600, "cash") };
    assert!(pay(&mut h, &round("PENDING", 1500), &right, &OPEN).is_ok());
}

#[test]
fn two_payments_closing_the_bill_mark_it_paid() {
    let mut h = hub();
    let (o1, _, _) = pay(&mut h, &round("PENDING", 1500), &input(600, "cash"), &OPEN).expect("first");
    let (o2, _, _) = pay(&mut h, &o1, &input(900, "card"), &OPEN).expect("second");
    assert_eq!(o2["payments"].as_array().map(Vec::len), Some(2));
    assert_eq!(o2.get("payment_status"), Some(&json!("paid")));
}

#[test]
fn a_payment_less_than_one_is_invalid() {
    let mut h = hub();
    assert_eq!(
        pay(&mut h, &round("PENDING", 1500), &input(0, "cash"), &OPEN),
        Err(Refused::Invalid("a payment is at least 1 minor unit".into()))
    );
    assert!(pay(&mut h, &round("PENDING", 1500), &input(1, "cash"), &OPEN).is_ok());
}

#[test]
fn a_payment_with_no_signer_is_refused_and_writes_nothing() {
    let mut h = hub();
    let hb = h.to_bytes();
    let i = PayIn { by: "  ".into(), ..input(600, "cash") };
    assert!(matches!(pay(&mut h, &round("PENDING", 1500), &i, &OPEN), Err(Refused::Invalid(_))));
    assert_eq!(h.to_bytes(), hb, "nothing written");
}

#[test]
fn a_payment_exceeding_the_total_is_refused() {
    let mut h = hub();
    let (o1, _, _) = pay(&mut h, &round("PENDING", 1500), &input(600, "cash"), &OPEN).expect("first");
    assert!(matches!(pay(&mut h, &o1, &input(1000, "cash"), &OPEN), Err(Refused::Conflict(_))));
    assert!(pay(&mut h, &o1, &input(900, "cash"), &OPEN).is_ok(), "exactly the rest lands");
}

#[test]
fn a_payment_on_a_settled_bill_is_refused() {
    let mut h = hub();
    let (o1, _, _) = pay(&mut h, &round("PENDING", 1500), &input(1500, "card"), &OPEN).expect("settles");
    assert!(matches!(pay(&mut h, &o1, &input(1, "cash"), &OPEN), Err(Refused::Conflict(_))));
}

#[test]
fn a_payment_on_another_venue_order_is_not_found() {
    let mut h = hub();
    let mut order = round("PENDING", 1500);
    order["location_id"] = json!("v2");
    assert_eq!(pay(&mut h, &order, &input(600, "cash"), &OPEN), Err(Refused::NotFound));
    assert_eq!(pay(&mut h, &round("PENDING", 1500), &input(600, "cash"), &OPEN).map(|_| ()), Ok(()));
}

/// Every method in the closed set lands; anything else, including " cash"
/// (which would otherwise slip past the till rule), is refused.
#[test]
fn the_closed_set_of_methods_and_nothing_else() {
    for m in ["cash", "card", "cheque", "transfer", "gift_card", "other"] {
        assert!(pay(&mut hub(), &round("PENDING", 1500), &input(1000, m), &OPEN).is_ok(), "{m}");
    }
    for m in ["bogus", " cash", "CASH"] {
        assert!(matches!(pay(&mut hub(), &round("PENDING", 1500), &input(1000, m), &SHUT), Err(Refused::Invalid(_))), "{m}");
    }
}

#[test]
fn payment_never_changes_the_round_money_fields() {
    let orig = round("PENDING", 1500);
    let (paid, _, _) = pay(&mut hub(), &orig, &input(600, "cash"), &OPEN).expect("lands");
    for k in ["subtotal", "discount", "total", "tip", "items"] {
        assert_eq!(paid[k], orig[k], "{k}");
    }
}

#[test]
fn a_split_payment_over_three_calls() {
    let mut h = hub();
    let (o1, _, _) = pay(&mut h, &round("PENDING", 1500), &input(500, "cash"), &OPEN).expect("first");
    assert_eq!(o1.get("payment_status"), None);
    let (o2, _, _) = pay(&mut h, &o1, &input(600, "cash"), &OPEN).expect("second");
    assert_eq!(o2.get("payment_status"), None);
    let (o3, _, _) = pay(&mut h, &o2, &input(400, "card"), &OPEN).expect("third");
    assert_eq!(o3.get("payment_status"), Some(&json!("paid")));
}

#[test]
fn a_cancelled_round_takes_no_payment_and_a_completed_one_does() {
    for s in ["CANCELLED", "REJECTED", "REFUNDING", "COMPENSATED_REFUND"] {
        let mut h = hub();
        let before = h.to_bytes();
        assert!(matches!(pay(&mut h, &round(s, 1500), &input(500, "cash"), &OPEN), Err(Refused::Conflict(_))), "{s}");
        assert_eq!(h.to_bytes(), before, "{s}: nothing written");
    }
    pay(&mut hub(), &round("DELIVERED", 1500), &input(500, "cash"), &OPEN).expect("a served round is paid");
}

/// P3-2: €20.00 in cash on a lek bill at 1 EUR = 97.50 ALL. The bill is
/// credited 1950 lek; the payment keeps what was handed over.
#[test]
fn a_euro_payment_on_a_lek_bill_is_credited_in_lek_at_its_rate() {
    let (o, _, _) = pay(&mut hub(), &round("PENDING", 5000), &eur(2000, Some(975_000)), &OPEN).expect("lands");
    let p = &o["payments"][0];
    assert_eq!(p["currency"], json!("EUR"));
    assert_eq!(p["amount"], json!(2000));
    assert_eq!(p["rate_ppm"], json!(975_000));
    assert_eq!(p["amount_in_order_currency"], json!(1950));
    assert_eq!(o["total"], json!(5000), "the invoice stays in lek");
}

#[test]
fn a_euro_payment_without_a_rate_is_invalid_and_writes_nothing() {
    let mut h = hub();
    let before = h.to_bytes();
    assert!(matches!(pay(&mut h, &round("PENDING", 5000), &eur(2000, None), &OPEN), Err(Refused::Invalid(_))));
    assert_eq!(h.to_bytes(), before);
}

/// Σ ≤ total and "paid" count the LEK a euro note is worth, not its cents.
/// €20.00 is 1950 lek against a 3000-lek bill; 1050 lek settles the rest
/// exactly, and one lek more is refused.
#[test]
fn the_bill_is_settled_in_the_orders_currency_across_two_currencies() {
    let mut h = hub();
    let (o1, _, _) = pay(&mut h, &round("PENDING", 3000), &eur(2000, Some(975_000)), &OPEN).expect("euro");
    assert!(matches!(pay(&mut h, &o1, &input(1051, "cash"), &OPEN), Err(Refused::Conflict(_))));
    let (o2, _, _) = pay(&mut h, &o1, &input(1050, "cash"), &OPEN).expect("lek");
    assert_eq!(o2.get("payment_status"), Some(&json!("paid")));
}

/// A euro that converts to more than the bill is refused BEFORE anything is
/// written: 2000 cents at 975 000 is 1950 lek, over a 1900 bill.
#[test]
fn a_euro_payment_worth_more_than_the_rest_is_refused() {
    assert!(matches!(pay(&mut hub(), &round("PENDING", 1900), &eur(2000, Some(975_000)), &OPEN), Err(Refused::Conflict(_))));
    assert!(pay(&mut hub(), &round("PENDING", 1950), &eur(2000, Some(975_000)), &OPEN).is_ok());
}

/// An order that names its own currency is settled in it, not the venue's.
#[test]
fn an_order_in_euro_takes_a_euro_payment_with_no_rate() {
    let mut o = round("PENDING", 1500);
    o["currency"] = json!("EUR");
    assert!(pay(&mut hub(), &o, &eur(1500, None), &OPEN).is_ok());
    assert!(pay(&mut hub(), &o, &input(1500, "cash"), &OPEN).is_ok(), "no currency named is the order's: EUR");
    let lek = PayIn { currency: Some("ALL".into()), ..input(1500, "cash") };
    assert!(matches!(pay(&mut hub(), &o, &lek, &OPEN), Err(Refused::Invalid(_))), "lek on a euro bill states a rate");
}

// ── §2.3 (P1-3): a tip taken at payment ─────────────────────────────────────

/// "KEEP THE CHANGE": 1500 owed, the guest hands 1700. `amount` is the bill's
/// share, `tip` the change; the round's tip and total rise together (law 3)
/// and the round is paid against the NEW total.
#[test]
fn a_tip_raises_tip_and_total_together_and_settles_the_round() {
    let mut h = hub();
    let i = PayIn { tip: Some(200), ..input(1500, "card") };
    let (order, body, _) = pay(&mut h, &round("READY", 1500), &i, &OPEN).expect("lands");
    assert_eq!((order["tip"].clone(), order["total"].clone()), (json!(200), json!(1700)));
    assert_eq!(order["subtotal"], json!(1500), "the lines do not move");
    assert_eq!(order["payments"][0]["amount"], json!(1500), "the drawer/bill share excludes the tip");
    assert_eq!(order["payments"][0]["tip"], json!(200));
    assert_eq!(order["payment_status"], json!("paid"));
    assert_eq!(settles(&order["payments"][0]), 1700, "a tip settles the total it raised");
    // Law 3 holds because the two terms move together: Δtotal == Δtip.
    assert_eq!(order["total"].as_i64().unwrap() - 1500, order["tip"].as_i64().unwrap() - 0);
    assert!(body.contains("\"tip\":200"));
}

/// Twins: a negative tip is refused and writes nothing; a tip on a payment
/// that would pay past the new total is still over-payment.
#[test]
fn a_negative_tip_or_an_overpaying_tip_is_refused_and_writes_nothing() {
    let mut h = hub();
    let before = h.to_bytes();
    let neg = PayIn { tip: Some(-1), ..input(600, "card") };
    assert!(matches!(pay(&mut h, &round("READY", 1500), &neg, &OPEN), Err(Refused::Invalid(_))));
    let over = PayIn { tip: Some(100), ..input(1600, "card") };
    assert!(matches!(pay(&mut h, &round("READY", 1500), &over, &OPEN), Err(Refused::Conflict(_))));
    assert_eq!(h.to_bytes(), before, "nothing written on refusal");
    let ok = PayIn { tip: Some(100), ..input(1500, "card") };
    assert!(pay(&mut h, &round("READY", 1500), &ok, &OPEN).is_ok(), "the positive twin");
}

/// No tip is exactly the old payment: total and tip untouched, no `tip` key.
#[test]
fn no_tip_leaves_the_total_alone() {
    let mut h = hub();
    let (order, _, _) = pay(&mut h, &round("READY", 1500), &input(1500, "card"), &OPEN).unwrap();
    assert_eq!((order["tip"].clone(), order["total"].clone()), (json!(0), json!(1500)));
    assert_eq!(order["payments"][0].get("tip"), None);
}

/// A wallet payment names its wallet, and only a wallet payment does.
#[test]
fn only_a_wallet_payment_names_a_wallet() {
    let mut h = hub();
    let bare = input(600, "wallet");
    assert!(matches!(pay(&mut h, &round("READY", 1500), &bare, &OPEN), Err(Refused::Invalid(_))));
    let card = PayIn { wallet: Some("u1".into()), ..input(600, "card") };
    assert!(matches!(pay(&mut h, &round("READY", 1500), &card, &OPEN), Err(Refused::Invalid(_))));
    let named = PayIn { wallet: Some("u1".into()), ..input(600, "wallet") };
    let (order, _, _) = pay(&mut h, &round("READY", 1500), &named, &OPEN).expect("the twin");
    assert_eq!(order["payments"][0]["wallet"], json!("u1"));
}
