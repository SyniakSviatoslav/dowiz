//! W-AUDIT 2026-09-27, through the REAL `decide` over real images.
//!
//! F2: a round the CARD took money for (the Stripe webhook's
//! `payment_status`/`amount_received`, never a `payments[]` entry) cannot be
//! rejected or cancelled: `refund::money_taken` sums every rail, and the
//! guard reads it. F9: the seq the tablets quote moves forward from the
//! order's newest event, never back to a clock that is behind it.

use super::{decide, AdvanceIn};
use crate::command::Refused;
use dowiz_hub::stock::{reservations_for, StockEvent, StockLog};
use dowiz_hub::EventKind;
use serde_json::{json, Value};

const ROLL: &str = r#"{"id":"p1","bom":[{"supply":"salmon","qty":40}]}"#;
const NOW: i64 = 1_700_000_100_000;

fn order(status: &str, extra: Value) -> String {
    let mut o = json!({
        "id": "o1", "order_id": "o1", "location_id": "sushi-durres", "status": status,
        "items": [{"product_id": "p1", "quantity": 2, "unit_price": 900, "name": "Sake"}],
        "subtotal": 1800, "total": 1800, "payment": "card",
        "contact": {"name": "C", "phone": "+355690000000"}, "created_at_ms": 1_700_000_000_000i64,
    });
    if let (Some(o), Some(e)) = (o.as_object_mut(), extra.as_object()) {
        for (k, v) in e {
            o.insert(k.clone(), v.clone());
        }
    }
    o.to_string()
}

fn images() -> (dowiz_hub::Hub, StockLog) {
    let hub = dowiz_hub::Hub::create_sized(64 * 1024).expect("hub");
    let mut stock = StockLog::create_sized(64 * 1024).expect("stock");
    stock.append(&StockEvent::Received { item: "salmon".into(), qty: 1000 }).unwrap();
    stock.append_all(&reservations_for("o1", &[(ROLL.into(), 2)])).unwrap();
    (hub, stock)
}

fn input(next: &str) -> AdvanceIn {
    AdvanceIn { order_id: "o1".into(), location_id: "sushi-durres".into(), next: next.into(), reason: None, now_ms: NOW }
}

/// F2. The webhook's record, and the older shape with no `amount_received`.
#[test]
fn a_card_paid_round_is_not_rejected_or_cancelled_without_its_money_recorded() {
    for paid in [json!({"payment_status": "paid", "amount_received": 1800}), json!({"payment_status": "paid"})] {
        for next in ["REJECTED", "CANCELLED"] {
            let (mut hub, mut stock) = images();
            let cur = order("PENDING", paid.clone());
            let r = decide(&mut hub, &mut stock, Some(&cur), &input(next));
            assert!(matches!(&r, Err(Refused::Conflict(m)) if m.contains("refund")), "{next} with {paid}: {r:?}");
            assert_eq!(hub.len(), 0, "a refusal appends nothing");
        }
    }
}

/// F2's twin: with no money on any rail the same edges are the kernel's to allow.
#[test]
fn an_unpaid_round_is_still_rejected() {
    let (mut hub, mut stock) = images();
    let cur = order("PENDING", json!({}));
    let merged = decide(&mut hub, &mut stock, Some(&cur), &input("REJECTED")).expect("nothing to refund");
    assert_eq!(merged["status"], json!("REJECTED"));
    assert_eq!(hub.len(), 1);
}

/// F9. The order's newest event sits at S; the advance carries a clock BEHIND
/// it (three writers in one coarse millisecond). The version must still move.
#[test]
fn the_advance_seq_moves_forward_from_the_newest_event_never_back_to_the_clock() {
    let (mut hub, mut stock) = images();
    let cur = order("CONFIRMED", json!({}));
    let s: u64 = (NOW as u64) + 100_000;
    hub.append(EventKind::Placed, "o1", &cur, s, [0u8; 32]).unwrap();
    decide(&mut hub, &mut stock, Some(&cur), &input("PREPARING")).expect("a legal edge");
    let newest = &hub.events()[0];
    assert_eq!(newest.kind, EventKind::Advanced);
    assert_eq!(newest.seq, s + 1, "next_seq(prev, now) = max(now, prev + 1); the clock {NOW} is behind {s}");
    assert_eq!(dowiz_hub::room::view::latest_seq(&hub, "o1"), s + 1);
}

/// F9's twin: a clock ahead of the newest event IS the next seq.
#[test]
fn a_clock_ahead_of_the_newest_event_is_the_seq() {
    let (mut hub, mut stock) = images();
    let cur = order("CONFIRMED", json!({}));
    hub.append(EventKind::Placed, "o1", &cur, 5, [0u8; 32]).unwrap();
    decide(&mut hub, &mut stock, Some(&cur), &input("PREPARING")).expect("a legal edge");
    assert_eq!(hub.events()[0].seq, NOW as u64);
}
