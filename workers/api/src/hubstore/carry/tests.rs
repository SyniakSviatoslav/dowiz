//! EVERY KEY A WRITER PUTS ON AN ORDER SURVIVES THE KERNEL'S TRANSITIONS.
//!
//! The table below is written from the WRITERS, not from `HUB_OWNED` — a test
//! that iterated the list would pass for every key the list forgot, which is
//! the defect. Each row names the file that writes the key. The order is then
//! walked through the REAL path (`command::advance::decide`: the kernel's
//! `apply_event_logic`, then `carry_over`, then the delta appended to a log),
//! PENDING → CONFIRMED → PREPARING → READY → PICKED_UP, and every key must come
//! out byte-equal, both on the merged order and on the order FOLDED from the
//! log (which is what every reader actually sees).

use serde_json::{json, Value};

/// (key, who writes it, a distinctive value).
fn written() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        // ── kernel-owned: `order_to_value` round-trips these itself ──
        ("id", "kernel place_order_at", json!("r1")),
        ("customer_id", "kernel place_order_at", json!("c-1")),
        ("created_at_ms", "kernel place_order_at", json!(1_790_000_000_000i64)),
        ("channel", "services/ordering/channel.rs", json!("ebills")),
        ("cash_pay_with", "kernel place_order_at", json!("5000")),
        ("subtotal", "kernel (recomputed from items)", json!(1500)),
        // ── hub-owned: placement ──
        ("items", "storefront.rs (name, vat_ppm on the line)", json!([
            {"product_id": "maki", "modifier_ids": [], "quantity": 2, "unit_price": 600, "name": "Maki", "vat_ppm": 200000},
            {"product_id": "beer", "modifier_ids": [], "quantity": 1, "unit_price": 300, "name": "Beer", "comped": true}
        ])),
        ("location_id", "storefront.rs", json!("v1")),
        ("contact", "storefront.rs", json!({"name": "Ana", "phone": "+355691234567"})),
        ("fulfilment", "storefront.rs / amend Op::Table", json!({"kind": "dine_in", "table": "7", "note": null, "address": null, "fee": 0})),
        ("payment", "storefront.rs", json!("cash")),
        ("crypto", "storefront.rs", json!({"wallet": null, "paid": false})),
        ("delivery_fee", "storefront.rs", json!(0)),
        ("tip", "storefront.rs", json!(100)),
        ("total", "storefront.rs / room_rules.rs", json!(1300)),
        ("discount", "command/place.rs / room_rules.rs", json!(300)),
        ("promo", "command/place.rs", json!({"code": "SUMMER", "discount": 300})),
        ("tax", "services/ordering/tax_block.rs", json!({"lines": [], "total": 1300})),
        ("placed_by", "storefront.rs (staffed)", json!("p1")),
        ("sitting_id", "storefront.rs (staffed)", json!("sit-000001")),
        ("scheduled_for_ms", "storefront.rs", json!(null)),
        // ── hub-owned: the room ──
        ("payments", "command/pay.rs", json!([{"amount": 1000, "method": "cash", "till_id": "t1", "by": "p1", "at": 1}])),
        ("payment_status", "command/pay.rs / stripe.rs / ebills/map.rs", json!("paid")),
        ("amended", "command/amend.rs / transfer.rs", json!([{"by": "p1", "at": 2, "ops": []}])),
        ("adjustments", "command/amend.rs / transfer.rs", json!([{"kind": "comp", "line": 1, "amount": 300}])),
        ("refund", "command/refund.rs", json!({"reason": "wrong_dish", "by": "p1", "at": 3})),
        ("kitchen", "command/kitchen_ack.rs", json!({"seen": {"by": "cook1", "at": 4}})),
        // ── hub-owned: payments and imports ──
        ("payment_intent", "stripe.rs", json!("pi_1")),
        ("amount_received", "stripe.rs", json!(1300)),
        ("stripe_event", "stripe.rs", json!("evt_fp")),
        ("currency", "ebills/map.rs", json!("ALL")),
        ("external", "ebills/map.rs", json!({"source": "ebills", "sale_id": 9, "uuid": "u-1"})),
        ("price_trusted", "ebills/map.rs", json!(true)),
        ("entered_by", "services/orders/aggregator.rs", json!("p_ana")),
        // ── hub-owned: delivery and notes ──
        ("courier_id", "command/assign.rs / courier.rs", json!("k1")),
        ("assigned_at_ms", "command/assign.rs", json!(5)),
        ("accepted_at_ms", "courier.rs", json!(6)),
        ("cash_collected", "courier.rs", json!(1300)),
        ("courier_note", "courier.rs", json!("gate code 12")),
        ("feedback", "services/orders/feedback.rs", json!({"text": "good", "at": 7})),
    ]
}

fn placed() -> Value {
    let mut o = json!({ "status": "PENDING" });
    for (k, _, v) in written() {
        o[k] = v;
    }
    o
}

const PATH: [&str; 4] = ["CONFIRMED", "PREPARING", "READY", "PICKED_UP"];

/// Walk the real transition path; answer the merged order and the log.
fn walk() -> (Value, dowiz_hub::Hub) {
    let mut hub = dowiz_hub::Hub::create_sized(256 * 1024).unwrap();
    let mut stock = dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap();
    let mut order = placed();
    hub.append(dowiz_hub::EventKind::Placed, "r1", &order.to_string(), 1, [0u8; 32]).unwrap();
    for (i, next) in PATH.iter().enumerate() {
        let input = crate::command::advance::AdvanceIn {
            order_id: "r1".into(),
            location_id: "v1".into(),
            next: (*next).into(),
            reason: None,
            now_ms: 1_790_000_000_000 + i as i64 + 1,
        };
        order = crate::command::advance::decide(&mut hub, &mut stock, Some(&order.to_string()), &input)
            .unwrap_or_else(|r| panic!("{next}: the kernel refused a legal edge: {r:?}"));
    }
    (order, hub)
}

/// The keys that did not come out byte-equal.
fn erased(after: &Value) -> Vec<String> {
    written()
        .into_iter()
        .filter(|(k, _, v)| after.get(*k).map(Value::to_string) != Some(v.to_string()))
        .map(|(k, who, _)| format!("{k} ({who})"))
        .collect()
}

#[test]
fn every_written_key_survives_four_transitions_on_the_merged_order() {
    let (merged, _) = walk();
    assert_eq!(merged["status"], json!("PICKED_UP"), "the walk reached its end");
    let lost = erased(&merged);
    assert!(lost.is_empty(), "ERASED by a status transition: {lost:?}");
}

/// What readers see is the FOLD of the log, not `decide`'s return value.
#[test]
fn every_written_key_survives_in_the_folded_log() {
    let (_, hub) = walk();
    let folded = crate::hubstore::orders_state(&hub).into_iter().next().expect("r1 is in the log");
    let order: Value = serde_json::from_str(&folded.order_json).unwrap();
    assert_eq!(order["status"], json!("PICKED_UP"));
    let lost = erased(&order);
    assert!(lost.is_empty(), "ERASED from the folded order: {lost:?}");
}

/// The twin: a key NO writer owns is not carried — the list is a list, not
/// "copy everything", which would carry a stale `eta` (computed at read time,
/// `live_eta.rs`) into the log for ever.
#[test]
fn a_key_nobody_owns_is_not_carried() {
    let mut old = placed();
    old["eta"] = json!({"minutes": 12});
    let mut updated = json!({"status": "CONFIRMED"});
    super::carry_over(&old, &mut updated);
    assert!(updated.get("eta").is_none());
    assert_eq!(updated["payments"], old["payments"], "while an owned key is");
}
