//! P11: what the assistants are shown carries the ORDER and not the PERSON.
//! The facts are built by the same functions the routes call, over a real
//! hub, and the payload is searched for every name and phone it was fed.

use super::*;
use dowiz_hub::{EventKind, Hub};

const LOC: &str = "venue_1";
const NOW: i64 = 1_800_000_000_000;

fn phone(i: u64) -> String {
    format!("+35569{:07}", 1_234_500 + i)
}

fn name(i: u64) -> String {
    format!("Arben{i} Hoxha{i}")
}

fn order(i: u64, status: &str, courier: &str) -> String {
    json!({
        "id": format!("o{i}"), "location_id": LOC, "status": status, "total": 1800,
        "created_at_ms": NOW - 600_000, "courier_id": courier, "payment": "cash",
        "items": [{ "name": "Maki", "qty": 2 }],
        "contact": { "name": name(i), "phone": phone(i) },
        "fulfilment": { "kind": "delivery", "note": "ring twice",
            "address": { "line": format!("Rruga {i}"), "note": "floor 3", "lat_udeg": 41_312_345, "lon_udeg": 19_445_678 } },
    })
    .to_string()
}

/// Ten live orders, every one with a name, a phone and a home.
fn hub() -> Hub {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    for i in 0..10u64 {
        h.append(EventKind::Placed, &format!("o{i}"), &order(i, "PREPARING", "c1"), i + 1, [0xA1; 32]).unwrap();
    }
    h
}

fn digit_run(s: &str) -> usize {
    let (mut best, mut run) = (0, 0);
    for c in s.chars() {
        run = if c.is_ascii_digit() { run + 1 } else { 0 };
        best = best.max(run);
    }
    best
}

fn no_person(text: &str, what: &str) {
    for i in 0..10u64 {
        assert!(!text.contains(&name(i)) && !text.contains(&format!("Arben{i}")), "{what} names customer {i}: {text}");
        assert!(!text.contains(&phone(i)[4..]), "{what} carries phone {i}: {text}");
    }
    assert!(!text.contains("contact"), "{what} has a contact field: {text}");
}

/// THE CHECK: the whole owner payload -- live orders, the menu and the graph
/// the question retrieves -- holds no name and no phone for a fixture of ten
/// orders, and no digit run a phone could hide in (`now_ms` is the one long
/// number, and it is taken out first). The orders are still THERE: ten of
/// them, with their money and their lines.
#[test]
fn the_owner_assistant_sees_ten_orders_and_no_person() {
    let cat = dowiz_hub::catalog::Catalog::create().unwrap();
    let labels = std::collections::HashMap::new();
    for q in ["which orders are late", "Arben3", "+355691234503", "Rruga 4"] {
        let facts = owner_facts(&hub(), &cat, &labels, LOC, q, NOW);
        assert_eq!(facts["live_orders"].as_array().map(Vec::len), Some(10), "{q}: the orders must stay");
        assert_eq!(facts["live_orders"][0]["total"], json!(1800));
        assert_eq!(facts["live_orders"][0]["items"][0]["name"], json!("Maki"));
        let mut text = facts.to_string();
        text = text.replace(&NOW.to_string(), "NOW");
        no_person(&text, q);
        assert!(!text.contains("Rruga"), "{q}: a home is a person: {text}");
        assert!(digit_run(&text) < 7, "{q}: a phone-length digit run: {text}");
    }
}

/// The twin: the courier keeps the address LINE of their own run (they are
/// going to that door) and nothing else of the person.
#[test]
fn the_courier_assistant_keeps_the_address_line_and_no_person() {
    let o: Value = serde_json::from_str(&order(3, "IN_DELIVERY", "c1")).unwrap();
    let f = courier_run_fact(&o, NOW);
    assert_eq!(f["address"], json!("Rruga 3"));
    assert_eq!(f["total"], json!(1800));
    let text = f.to_string();
    no_person(&text, "courier fact");
    assert!(!text.contains("floor 3") && !text.contains("41312345"), "parts, note, coordinates: {text}");
}

/// The owner's single-order fact, for the record: the order's fields stay.
#[test]
fn the_owner_fact_keeps_what_late_and_who_carries_are_answered_from() {
    let o: Value = serde_json::from_str(&order(1, "READY", "c9")).unwrap();
    let f = owner_order_fact(&o, NOW);
    assert_eq!(f["courier_id"], json!("c9"));
    assert_eq!(f["waiting_minutes"], json!(10));
    assert_eq!(f["fulfilment"], json!("delivery"));
    assert_eq!(f.get("address"), None);
    assert_eq!(f.get("contact"), None);
}
