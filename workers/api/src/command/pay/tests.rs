//! G5 (BLUEPRINT-POS-THE-ROOM §6): the payment's refusals, each beside the
//! positive twin that proves the refusal is not the only thing it can do.

use super::*;

const NOW: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_789_999_000_000;

fn round(status: &str, total: i64) -> Value {
    json!({
        "id": "r1", "status": status, "location_id": "v1",
        "items": [
            {"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"},
        ],
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
        now_ms: NOW,
    }
}

fn hub() -> dowiz_hub::Hub {
    dowiz_hub::Hub::create_sized(64 * 1024).unwrap()
}

/// THE POSITIVE TWIN: a payment of 600 lands on a 1500 round,
/// the order records it but stays unpaid, and the signer is on the record.
#[test]
fn a_legal_partial_payment_lands() {
    let mut h = hub();
    let (order, body, seq) =
        decide(&mut h, Some(&view(&round("PENDING", 1500))), &input(600, "cash"))
            .expect("lands");
    assert_eq!(order["payments"][0]["amount"], json!(600));
    assert_eq!(order["payments"][0]["by"], json!("p1"));
    assert_eq!(order["payments"][0]["method"], json!("cash"));
    assert_eq!(order.get("payment_status"), None, "not paid yet");
    assert_eq!(order["total"], json!(1500), "total unchanged");
    assert_eq!(order["subtotal"], json!(1500), "subtotal unchanged");
    assert!(seq > SEQ, "the version moves");
    assert!(body.contains("\"_d\":true"), "a delta, not a snapshot");
    assert_eq!(h.events()[0].kind, dowiz_hub::EventKind::Paid);
}

/// THE POSITIVE TWIN: two payments totaling the bill mark it paid.
#[test]
fn two_payments_closing_the_bill_mark_it_paid() {
    let mut h = hub();
    let order_600 =
        decide(&mut h, Some(&view(&round("PENDING", 1500))), &input(600, "cash"))
            .expect("first lands");
    let order_900 = decide(
        &mut h,
        Some(&view(&serde_json::from_str::<Value>(&order_600.0.to_string()).unwrap())),
        &input(900, "card"),
    )
    .expect("second lands");
    assert_eq!(order_900.0["payments"].as_array().map(|a| a.len()), Some(2));
    assert_eq!(order_900.0.get("payment_status"), Some(&json!("paid")));
}

/// G5: amount < 1 is rejected
#[test]
fn a_payment_less_than_one_is_invalid() {
    let mut h = hub();
    assert_eq!(
        decide(&mut h, Some(&view(&round("PENDING", 1500))), &input(0, "cash")),
        Err(Refused::Invalid("a payment is at least 1 minor unit".into()))
    );
}

/// G5: no signer is rejected
#[test]
fn a_payment_with_no_signer_is_refused_and_writes_nothing() {
    let mut h = hub();
    let hb = h.to_bytes();
    let mut i = input(600, "cash");
    i.by = "  ".into();
    let err = decide(&mut h, Some(&view(&round("PENDING", 1500))), &i);
    assert!(matches!(err, Err(Refused::Invalid(_))));
    assert_eq!(h.to_bytes(), hb, "nothing written");
}

/// G5: Σ payments > total is a Conflict
#[test]
fn a_payment_exceeding_the_total_is_refused() {
    let mut h = hub();
    let round_1500 = round("PENDING", 1500);
    let order_600 = decide(&mut h, Some(&view(&round_1500)), &input(600, "cash")).expect("first");
    let order_update = view(&order_600.0);
    let result = decide(&mut h, Some(&order_update), &input(1000, "cash"));
    assert!(matches!(result, Err(Refused::Conflict(_))));
}

/// G5: a payment succeeding the overpayment (600 + 900 = 1500) is refused
/// if a third arrives.
#[test]
fn a_third_payment_on_a_completed_bill_is_refused() {
    let mut h = hub();
    let order_600 =
        decide(&mut h, Some(&view(&round("PENDING", 1500))), &input(600, "cash"))
            .expect("first");
    let order_900 = decide(
        &mut h,
        Some(&view(&order_600.0)),
        &input(900, "card"),
    )
    .expect("second: closes");
    let result = decide(&mut h, Some(&view(&order_900.0)), &input(1, "cash"));
    assert!(matches!(result, Err(Refused::Conflict(_))));
}

/// G5: another venue's order is NotFound
#[test]
fn a_payment_on_another_venue_order_is_not_found() {
    let mut h = hub();
    let mut order = round("PENDING", 1500);
    order["location_id"] = json!("v2");
    let result = decide(&mut h, Some(&view(&order)), &input(600, "cash"));
    assert_eq!(result, Err(Refused::NotFound));
}

/// G5: an invalid method is rejected
#[test]
fn an_invalid_payment_method_is_rejected() {
    let mut h = hub();
    let result = decide(&mut h, Some(&view(&round("PENDING", 1500))), &input(600, "bogus_method"));
    assert!(matches!(result, Err(Refused::Invalid(_))));
}

/// LAW 3 UNCHANGED: payment never changes subtotal, discount, or total
#[test]
fn payment_never_changes_the_round_money_fields() {
    let mut h = hub();
    let orig = round("PENDING", 1500);
    let orig_subtotal = orig["subtotal"].as_i64().unwrap();
    let orig_discount = orig["discount"].as_i64().unwrap();
    let orig_total = orig["total"].as_i64().unwrap();

    let (paid, _, _) = decide(&mut h, Some(&view(&orig)), &input(600, "cash")).expect("lands");
    assert_eq!(paid["subtotal"], json!(orig_subtotal));
    assert_eq!(paid["discount"], json!(orig_discount));
    assert_eq!(paid["total"], json!(orig_total));
}

/// G5: the split: a partial, then another partial, then exactly closed.
#[test]
fn a_split_payment_over_three_calls() {
    let mut h = hub();
    let o1 = decide(&mut h, Some(&view(&round("PENDING", 1500))), &input(500, "cash"))
        .expect("first");
    assert_eq!(o1.0.get("payment_status"), None, "not paid at 500");

    let o2 = decide(&mut h, Some(&view(&o1.0)), &input(600, "cash"))
        .expect("second");
    assert_eq!(o2.0.get("payment_status"), None, "not paid at 1100");

    let o3 = decide(&mut h, Some(&view(&o2.0)), &input(400, "card"))
        .expect("third");
    assert_eq!(o3.0.get("payment_status"), Some(&json!("paid")), "paid at 1500");
}

/// A sitting with multiple orders accumulates payments across them.
/// (Each order gets its own payments array; this tests one order's accumulation.)
#[test]
fn multiple_payments_accumulate_in_the_payments_array() {
    let mut h = hub();
    let o1 = decide(&mut h, Some(&view(&round("PENDING", 1000))), &input(300, "cash"))
        .expect("first");
    assert_eq!(o1.0["payments"].as_array().map(|a| a.len()), Some(1));

    let o2 = decide(&mut h, Some(&view(&o1.0)), &input(400, "cash"))
        .expect("second");
    assert_eq!(o2.0["payments"].as_array().map(|a| a.len()), Some(2));

    let o3 = decide(&mut h, Some(&view(&o2.0)), &input(300, "card"))
        .expect("third");
    assert_eq!(o3.0["payments"].as_array().map(|a| a.len()), Some(3));
    assert_eq!(o3.0.get("payment_status"), Some(&json!("paid")));
}

/// Valid methods: cash, card, cheque, transfer, gift_card, other
#[test]
fn all_valid_payment_methods_are_accepted() {
    let mut h = hub();
    for method in &["cash", "card", "cheque", "transfer", "gift_card", "other"] {
        let mut h_fresh = hub();
        let result =
            decide(&mut h_fresh, Some(&view(&round("PENDING", 1500))), &input(1000, method));
        assert!(
            result.is_ok(),
            "method {} should be accepted",
            method
        );
    }
}

/// A cancelled or refunding round takes no payment; a completed one does —
/// a dine-in bill is settled after the food.
#[test]
fn a_cancelled_round_takes_no_payment_and_a_completed_one_does() {
    for s in ["CANCELLED", "REJECTED", "REFUNDING", "COMPENSATED_REFUND"] {
        let mut h = hub();
        let before = h.to_bytes();
        let r = decide(&mut h, Some(&view(&round(s, 1500))), &input(500, "cash"));
        assert!(matches!(r, Err(Refused::Conflict(_))), "{s} must refuse");
        assert_eq!(h.to_bytes(), before, "{s}: nothing written");
    }
    let mut h = hub();
    decide(&mut h, Some(&view(&round("DELIVERED", 1500))), &input(500, "cash")).expect("a served round is paid");
}
