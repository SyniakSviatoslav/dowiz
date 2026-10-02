//! R2's two pure halves of the live estimate's read: which products to ask the
//! object for, and the cooking time out of what it answered.

use super::*;
use std::collections::HashMap;

#[test]
fn only_live_orders_lines_without_their_own_time_are_asked_for() {
    let live = json!({"status": "PREPARING", "items": [
        {"product_id": "p_maki", "quantity": 2},
        {"product_id": "p_miso", "cookingMin": 4},
        {"product_id": "p_maki"},
        {"quantity": 1}
    ]});
    let other = json!({"status": "PENDING", "items": [{"product_id": "p_soup"}]});
    let over = json!({"status": "DELIVERED", "items": [{"product_id": "p_done"}]});
    let none = json!({"status": "CONFIRMED"});
    assert_eq!(products_needed([&live, &other, &over, &none]), vec!["p_maki", "p_soup"], "once each, in order");
    assert!(products_needed([&over]).is_empty(), "a finished order asks for nothing");
    assert!(products_needed(std::iter::empty()).is_empty());
}

#[test]
fn a_cooking_time_is_read_off_the_answered_product_or_is_absent() {
    let mut p: HashMap<String, Value> = HashMap::new();
    p.insert("p_maki".into(), json!({"cookingMin": 12}));
    p.insert("p_bare".into(), json!({"name": "Bare"}));
    assert_eq!(cooking_in(&p, "p_maki"), Some(12));
    assert_eq!(cooking_in(&p, "p_bare"), None, "a product that says nothing is the venue's default, not zero");
    assert_eq!(cooking_in(&p, "p_gone"), None);
}

/// The estimate reads the answered products exactly as it read the catalogue:
/// a line without its own time takes the product's.
#[test]
fn the_estimate_takes_the_answered_products_time() {
    let mut p: HashMap<String, Value> = HashMap::new();
    p.insert("p_slow".into(), json!({"cookingMin": 40}));
    let order = json!({"status": "PENDING", "created_at_ms": 0, "fulfilment": {"kind": "pickup"},
        "items": [{"product_id": "p_slow", "quantity": 1}]});
    let loc = json!({});
    let k = crate::eta::profile_of(&loc);
    let with = estimate(&order, &loc, &k, &|id: &str| cooking_in(&p, id), &[], &[], 0).unwrap();
    let without = estimate(&order, &loc, &k, &|_: &str| None, &[], &[], 0).unwrap();
    assert!(with["parts"]["prepLeftMin"].as_u64() > without["parts"]["prepLeftMin"].as_u64(), "{with} vs {without}");
}

/// ORDER FOR LATER (N4.4): an order for 19:00 placed at noon is not "18-24
/// minutes". The estimate is anchored to the chosen hour, and the hour once
/// come, the kitchen's own arithmetic takes over.
#[test]
fn an_order_for_later_is_estimated_at_its_hour_and_not_before() {
    let now = 1_000_000_000;
    let in_six_hours = now + 6 * 60 * 60 * 1000;
    let order = json!({"status": "CONFIRMED", "created_at_ms": now, "fulfilment": {"kind": "pickup"},
        "scheduled_for_ms": in_six_hours, "items": [{"product_id": "p", "cookingMin": 10, "quantity": 1}]});
    let loc = json!({});
    let k = crate::eta::profile_of(&loc);
    let e = estimate(&order, &loc, &k, &|_: &str| None, &[], &[], now).unwrap();
    assert_eq!(e["scheduledForMs"], json!(in_six_hours), "{e}");
    assert!(e["arriveAtMs"].as_i64().unwrap() >= in_six_hours, "not before the hour: {e}");
    assert_eq!(e["minMin"], json!(360), "{e}");
    assert!(e["maxMin"].as_u64().unwrap() <= 360 + 5, "the spread is the handover's, not 25% of six hours: {e}");
    // Its hour come, the same order is estimated like any other.
    let due = estimate(&order, &loc, &k, &|_: &str| None, &[], &[], in_six_hours + 1).unwrap();
    assert!(due["scheduledForMs"].is_null(), "{due}");
    assert!(due["minMin"].as_u64().unwrap() < 60, "{due}");
    // An order for now is untouched by the rule.
    let plain = json!({"status": "CONFIRMED", "created_at_ms": now, "fulfilment": {"kind": "pickup"},
        "items": [{"product_id": "p", "cookingMin": 10, "quantity": 1}]});
    let p = estimate(&plain, &loc, &k, &|_: &str| None, &[], &[], now).unwrap();
    assert!(p["scheduledForMs"].is_null() && p["minMin"].as_u64().unwrap() < 60, "{p}");
}
