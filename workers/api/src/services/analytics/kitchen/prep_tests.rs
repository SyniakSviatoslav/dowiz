//! R3 (W-PF2): the kitchen's numbers through semi-finished products. A dish
//! whose recipe names a ПФ counts the RAW items the sale takes, exactly and
//! rounded once over the window -- not the ПФ as one ingredient.

use super::sales::{fold, portion_cost, Supply, Window};
use super::*;
use serde_json::json;

fn venue() -> dowiz_hub::catalog::Catalog {
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    let raw = |id: &str, unit: &str, cost: i64| json!({ "id": id, "name": id, "unit": unit, "kind": "food_ingredient", "costPerBasis": cost }).to_string();
    cat.set_supply("vinegar", &raw("vinegar", "ml", 30));
    cat.set_supply("salt", &raw("salt", "g", 5));
    cat.set_supply("sugar", &raw("sugar", "g", 15));
    cat.set_supply("rice-dry", &raw("rice-dry", "g", 20));
    cat.set_supply("water", r#"{"id":"water","name":"Water","unit":"ml","kind":"food_ingredient","untracked":true}"#);
    cat.set_supply("mitsukan", r#"{"id":"mitsukan","name":"Mitsukan","unit":"g","kind":"prep","card":{"lines":[{"item":"vinegar","qty":800},{"item":"salt","qty":50},{"item":"sugar","qty":150}],"yield":1000}}"#);
    cat.set_supply("rice-seasoned", r#"{"id":"rice-seasoned","name":"Rice seasoned","unit":"g","kind":"prep","card":{"lines":[{"item":"rice-dry","qty":1000},{"item":"water","qty":1100},{"item":"mitsukan","qty":250}],"yield":2100}}"#);
    cat.set_product("philadelphia", r#"{"id":"philadelphia","name":"Philadelphia","price":650,"bom":[{"supply":"rice-seasoned","qty":130}]}"#);
    cat.set_product("maki", r#"{"id":"maki","name":"Maki","price":300,"bom":[{"supply":"rice-dry","qty":90}]}"#);
    cat
}

fn one_day() -> Window {
    Window { starts: vec![0], end: 1_000, days: vec![20260929] }
}

fn sold(n: i64, dish: &str) -> Vec<Value> {
    (0..n).map(|i| json!({ "id": format!("o{i}"), "created_at_ms": 10, "status": "DELIVERED", "items": [{ "product_id": dish, "quantity": 1, "unit_price": 650 }] })).collect()
}

#[test]
fn a_semi_finished_line_counts_its_raw_items_not_itself() {
    let cat = venue();
    let dishes = dishes_of(&cat);
    assert_eq!(dishes["philadelphia"].leaves, vec![("rice-dry".into(), 61_904_762), ("salt".into(), 773_810), ("sugar".into(), 2_321_429), ("vinegar".into(), 12_380_952)]);
    assert!(dishes["maki"].leaves.is_empty(), "the twin: a raw recipe keeps its lines");
    // A thousand rolls: 774 g of salt (exact 773.81), not 1 000 and not the ПФ as one item.
    let s = fold(&sold(1000, "philadelphia"), &dishes, &one_day());
    assert!(!s.uses.contains_key("rice-seasoned"), "the ПФ is not an ingredient of the report");
    assert_eq!((s.uses["salt"].qty, s.uses["salt"].by_day.clone()), (774, vec![774]));
    assert_eq!(s.uses["rice-dry"].qty, 61_905);
    assert!(!s.uses.contains_key("water"), "untracked");
    let m = fold(&sold(3, "maki"), &dishes, &one_day());
    assert_eq!(m.uses["rice-dry"].qty, 270);
}

#[test]
fn a_semi_finished_portion_has_a_cost_through_its_tree() {
    let cat = venue();
    let dishes = dishes_of(&cat);
    let supplies: HashMap<String, Supply> = supplies_of(&cat);
    let book = dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap().cost_book();
    // List prices per 100: rice 61.904762 g x 0.20 + vinegar 12.380952 x 0.30 + salt 0.773810 x 0.05 + sugar 2.321429 x 0.15
    // = 12.381 + 3.714 + 0.039 + 0.348 = 16.48 -> 16.
    assert_eq!(portion_cost(&dishes["philadelphia"], &supplies, &book), Some(16));
    assert_eq!(portion_cost(&dishes["maki"], &supplies, &book), Some(18), "the twin: 90 g at 0.20");
}
