//! I0 (operator, 2026-09-26): "every order must really reduce stock". Each
//! way an order can leave the kitchen is walked through the REAL `decide`
//! over real images, and each ends with the reservation CONSUMED -- never
//! stranded, never released, never taken twice.
//!
//! The paths (FSM, `crates/dowiz-core/src/order_machine.rs`):
//!   delivery      CONFIRMED -> PREPARING -> READY -> IN_DELIVERY -> DELIVERED
//!   skip-the-prep CONFIRMED -> IN_DELIVERY -> DELIVERED (a courier takes a
//!                 CONFIRMED order: `assign::HANDABLE` includes CONFIRMED)
//!   counter       CONFIRMED -> PREPARING -> READY -> PICKED_UP
//! A table round, an aggregator (Wolt) order and a guest round move through
//! this same `decide` (owner.rs, services/orders/room/guest_round.rs), and the
//! courier's pickup now does too (courier.rs). An ebills till sale is a
//! `Served` at import and never holds (ebills/import.rs).

use super::{decide, AdvanceIn};
use dowiz_hub::stock::{reservations_for, StockEvent, StockLog};
use serde_json::{json, Value};

const ROLL: &str = r#"{"id":"p1","bom":[{"supply":"salmon","qty":40},{"supply":"rice","qty":90}]}"#;

fn order(status: &str) -> String {
    json!({
        "id": "o1", "order_id": "o1", "location_id": "sushi-durres", "status": status,
        "items": [{"product_id": "p1", "quantity": 2, "unit_price": 900, "name": "Sake"}],
        "subtotal": 1800, "total": 1800, "payment": "cash",
        "contact": {"name": "C", "phone": "+355690000000"}, "created_at_ms": 1_700_000_000_000i64,
    })
    .to_string()
}

/// Salmon counted at 1000 g; rice never counted. Order o1 holds 2 rolls.
fn images() -> (dowiz_hub::Hub, StockLog) {
    let hub = dowiz_hub::Hub::create_sized(64 * 1024).expect("hub");
    let mut stock = StockLog::create_sized(64 * 1024).expect("stock");
    stock.append(&StockEvent::Received { item: "salmon".into(), qty: 1000 }).unwrap();
    stock.append_all(&reservations_for("o1", &[(ROLL.into(), 2)])).unwrap();
    (hub, stock)
}

/// Walk `path` from CONFIRMED, feeding each merged order to the next step.
fn walk(path: &[&str]) -> StockLog {
    let (mut hub, mut stock) = images();
    let mut current = order("CONFIRMED");
    for next in path {
        let input = AdvanceIn {
            order_id: "o1".into(), location_id: "sushi-durres".into(), next: (*next).into(), reason: None, now_ms: 1_700_000_100_000,
        };
        let merged: Value = decide(&mut hub, &mut stock, Some(&current), &input).unwrap_or_else(|e| panic!("{next}: {e:?}"));
        current = merged.to_string();
    }
    stock
}

fn consumed(stock: &StockLog) -> Vec<(String, i64)> {
    stock
        .events()
        .into_iter()
        .filter_map(|e| match e {
            StockEvent::Consumed { item, qty, .. } => Some((item, qty)),
            _ => None,
        })
        .collect()
}

/// Every path: the hold is consumed exactly once -- 80 g salmon off a counted
/// 1000, and 180 g rice off an uncounted shelf, which goes NEGATIVE ("needs a
/// count") rather than staying at a zero that hides the use.
#[test]
fn every_fulfilment_path_consumes_exactly_once() {
    for path in [
        &["PREPARING", "READY", "IN_DELIVERY", "DELIVERED"][..],
        &["IN_DELIVERY", "DELIVERED"][..],
        &["PREPARING", "READY", "PICKED_UP"][..],
    ] {
        let stock = walk(path);
        let led = stock.ledger().unwrap();
        assert!(led.stranded().is_empty(), "{path:?} left a hold behind");
        assert_eq!(consumed(&stock), vec![("rice".into(), 180), ("salmon".into(), 80)], "{path:?}: once, not per step");
        assert_eq!(led.level("salmon").on_hand, 920, "{path:?}");
        assert_eq!(led.level("rice").on_hand, -180, "{path:?}: uncounted, and still reduced");
    }
}

/// THE DEFECT, named: CONFIRMED -> IN_DELIVERY consumed nothing before this
/// change -- the first step alone now consumes.
#[test]
fn the_edge_that_skips_preparing_consumes() {
    let stock = walk(&["IN_DELIVERY"]);
    assert_eq!(consumed(&stock).len(), 2);
    assert!(stock.ledger().unwrap().stranded().is_empty());
}

/// Twin: an order that is not cooked releases, and consumes nothing.
#[test]
fn a_cancelled_order_releases_and_consumes_nothing() {
    let (mut hub, mut stock) = images();
    let input = AdvanceIn {
        order_id: "o1".into(), location_id: "sushi-durres".into(), next: "REJECTED".into(), reason: None, now_ms: 1,
    };
    decide(&mut hub, &mut stock, Some(&order("PENDING")), &input).expect("a pending order may be rejected");
    assert!(consumed(&stock).is_empty());
    let led = stock.ledger().unwrap();
    assert!(led.stranded().is_empty());
    assert_eq!((led.level("salmon").on_hand, led.level("rice").on_hand), (1000, 0));
}
