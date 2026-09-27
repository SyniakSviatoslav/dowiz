//! Sales x recipes, by day: the theoretical use.

use super::*;
use serde_json::json;

fn w() -> Window {
    Window { starts: vec![0, 100, 200], end: 300, days: vec![20260924, 20260925, 20260926] }
}
fn dishes() -> HashMap<String, Dish> {
    let line = |s: &str, qty, g, n, o| DishLine { supply: s.into(), qty, gross_g: Some(g), net_g: Some(n), out_g: Some(o) };
    HashMap::from([
        ("sake".to_string(), Dish { id: "sake".into(), name: "Sake".into(), lines: vec![line("salmon", 100, 100, 55, 50), line("rice", 90, 90, 90, 200)] }),
        ("cola".to_string(), Dish { id: "cola".into(), name: "Cola".into(), lines: vec![] }),
    ])
}
fn order(id: &str, at: i64, status: &str, items: Value) -> Value {
    json!({ "id": id, "created_at_ms": at, "status": status, "items": items })
}

#[test]
fn sold_portions_draw_their_recipe_on_their_day() {
    let orders = vec![
        order("o1", 10, "DELIVERED", json!([{ "product_id": "sake", "quantity": 2, "unit_price": 900 }, { "product_id": "cola", "quantity": 1, "unit_price": 200 }])),
        order("o2", 150, "DELIVERED", json!([{ "product_id": "sake", "quantity": 1, "unit_price": 900 }])),
        order("o3", 160, "REJECTED", json!([{ "product_id": "sake", "quantity": 5, "unit_price": 900 }])),
        order("o4", 999, "DELIVERED", json!([{ "product_id": "sake", "quantity": 5, "unit_price": 900 }])),
    ];
    let s = fold(&orders, &dishes(), &w());
    assert_eq!(s.days.iter().map(|d| (d.orders, d.revenue)).collect::<Vec<_>>(), vec![(1, 2000), (1, 900), (0, 0)]);
    let sake = &s.dishes["sake"];
    assert_eq!((sake.sold, sake.revenue, sake.by_day.clone()), (3, 2700, vec![2, 1, 0]), "the refused and the out-of-window are not sales");
    let salmon = &s.uses["salmon"];
    assert_eq!((salmon.qty, salmon.gross_g, salmon.net_g, salmon.out_g, salmon.by_day.clone()), (300, 300, 165, 150, vec![200, 100, 0]));
    assert_eq!(s.uses["rice"].out_g, 600, "rice grows");
    assert_eq!(s.unmodelled, 1, "one cola, no recipe");
    assert_eq!(s.placed_at.get("o3"), Some(&160), "every order is dated, sold or not");
}

#[test]
fn a_portion_costs_its_gross_at_the_average_else_the_list_price() {
    use dowiz_hub::stock::meta::Meta;
    let sup = |id: &str, list| Supply { id: id.into(), name: id.into(), unit: "g".into(), basis: 100, list_cost: list, clean_pm: 1000, cook_pm: 1000 };
    let supplies = HashMap::from([("salmon".to_string(), sup("salmon", Some(180))), ("rice".to_string(), sup("rice", None))]);
    let mut log = dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap();
    let book = log.cost_book();
    assert_eq!(portion_cost(&dishes()["sake"], &supplies, &book), None, "rice has no price at all");
    log.receive_with("rice", 1000, &Meta { unit_cost: Some(200), per: Some(1000), ..Meta::default() }).unwrap();
    let book = log.cost_book();
    assert_eq!(portion_cost(&dishes()["sake"], &supplies, &book), Some(180 + 18), "salmon at list, rice at its average");
    assert_eq!(portion_cost(&dishes()["cola"], &supplies, &book), None, "no recipe is no cost");
}

/// R4: a line stamped at placement is costed at what it cost THEN; an order
/// from before stamps falls back to today's portion cost; a dish with no
/// cost today is costed only when every portion was stamped.
#[test]
fn stamped_lines_keep_their_cost_and_old_ones_fall_back_to_today() {
    let orders = vec![
        order("new", 10, "DELIVERED", json!([{ "product_id": "sake", "quantity": 2, "unit_price": 900, "unit_cost": 300, "cost_at": 7 }])),
        order("old", 150, "DELIVERED", json!([{ "product_id": "sake", "quantity": 1, "unit_price": 900 }])),
    ];
    let s = fold(&orders, &dishes(), &w());
    let sake = &s.dishes["sake"];
    assert_eq!((sake.stamped, sake.stamped_cogs, sake.stamped_by_day.clone()), (2, 600, vec![2, 0, 0]));
    // Today a portion costs 450: the stamped two stay at 300, the old one takes 450.
    assert_eq!(cogs_of(sake, Some(450)), (Some(600 + 450), vec![600, 450, 0]));
    // With no cost today the old portion has none: the whole is unknown, the stamped days still count.
    assert_eq!(cogs_of(sake, None), (None, vec![600, 0, 0]));
    // Twin: every portion stamped needs no cost today.
    let only_new = fold(&orders[..1], &dishes(), &w());
    assert_eq!(cogs_of(&only_new.dishes["sake"], None), (Some(600), vec![600, 0, 0]));
    // A row built without the stamped columns reads as unstamped.
    let bare = DishRow { sold: 1, by_day: vec![1], ..DishRow::default() };
    assert_eq!(cogs_of(&bare, Some(5)), (Some(5), vec![5]));
}
