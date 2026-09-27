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
