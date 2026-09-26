//! What the kitchen's assistant is shown: the tickets, the menu, the shelf --
//! and never the customer or the money. Every refusal has a positive twin.

use super::*;

const PHONE: &str = "+355691234567";
const STREET: &str = "Rruga Taulantia 14";

fn order(id: &str, status: &str, venue: &str) -> (String, String) {
    (id.to_string(), json!({
        "id": id, "status": status, "location_id": venue, "created_at_ms": 60_000,
        "total": 2400, "payment": "card", "courier_id": "c9",
        "contact": { "name": "Arta", "phone": PHONE },
        "fulfilment": { "kind": "delivery", "table": null, "address": { "line": STREET } },
        "items": [{ "product_id": "p1", "name": "Dragon roll", "quantity": 2, "unit_price": 1000, "station": "bar" }],
    }).to_string())
}

fn facts() -> Value {
    let orders = vec![order("o1", "PREPARING", "v"), order("o2", "DELIVERED", "v"), order("o3", "PENDING", "w")];
    let products = vec![
        ("p1".to_string(), json!({ "name": "Dragon roll", "available": true, "price": 1000 }).to_string()),
        ("p2".to_string(), json!({ "name": "Tuna nigiri", "available": false, "unavailableNote": "no tuna" }).to_string()),
    ];
    let supplies = vec![("salmon".to_string(), json!({ "name": "Salmon fillet", "costPerBasis": 900 }).to_string())];
    kitchen_facts(&orders, &products, &supplies, &[("salmon".into(), 1200, 300), ("rice".into(), 5, 9)], "v", 11 * 60_000)
}

#[test]
fn the_kitchen_is_never_shown_the_customer_the_money_or_the_courier() {
    let s = facts().to_string();
    for leak in [PHONE, STREET, "Arta", "\"total\"", "unit_price", "payment", "\"c9\"", "costPerBasis", "\"price\""] {
        assert!(!s.contains(leak), "{leak} reached the kitchen's model: {s}");
    }
}

#[test]
fn the_kitchen_is_shown_its_tickets_menu_and_shelf() {
    let f = facts();
    let t = &f["open_tickets"];
    assert_eq!(t.as_array().map(Vec::len), Some(1), "one open ticket at this venue: {t}");
    assert_eq!(t[0]["ticket"], "o1");
    assert_eq!(t[0]["waiting_minutes"], 10);
    assert_eq!(t[0]["items"][0], json!({ "dish": "Dragon roll", "qty": 2, "station": "bar" }));
    assert_eq!(t[0]["kind"], "delivery");
    assert_eq!(f["off_the_menu"], json!([{ "dish": "Tuna nigiri", "why": "no tuna" }]));
    assert_eq!(f["shelf"][0], json!({ "ingredient": "Salmon fillet", "id": "salmon", "on_hand": 1200, "reserved": 300, "available": 900 }));
    assert_eq!(f["shelf"][1]["ingredient"], "rice", "an unnamed supply is called by its id");
    assert_eq!(f["shelf"][1]["available"], 0, "over-reserved is none left, not a negative");
}

#[test]
fn the_kitchen_prompt_forbids_invention_and_the_askers_are_the_kitchen_words() {
    assert!(SYSTEM_KITCHEN.contains("Never invent"));
    assert_eq!(ASKERS, [Cap::Advance, Cap::Catalog, Cap::Stock]);
    let waiter = dowiz_hub::caps::Preset::Waiter.caps();
    assert!(!ASKERS.iter().any(|c| waiter.allows(*c)), "a waiter is not a kitchen asker");
    let kitchen = dowiz_hub::caps::Preset::Kitchen.caps();
    assert!(ASKERS.iter().all(|c| kitchen.allows(*c)));
}
