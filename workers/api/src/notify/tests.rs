//! The ticket text, and what of the person it carries (P11).
use super::*;

#[test]
fn lek_has_no_minor_unit_and_euro_has_two() {
    assert_eq!(money_text(1500, "ALL"), "1500 ALL");
    assert_eq!(money_text(981, "EUR"), "9.81 EUR");
    assert_eq!(money_text(-5, "EUR"), "-0.05 EUR");
}

#[test]
fn order_text_carries_the_total_and_every_line() {
    let env = json!({ "id": "abcdefghijkl", "contact": { "name": "Ana", "phone": "+355" },
        "fulfilment": { "kind": "delivery", "address": { "line": "Rruga 1" } },
        "payment": "cash", "total": 1800, "delivery_fee": 300 });
    let lines = [LineOut { name: "Sake".into(), quantity: 2, unit_price: 750 }];
    let t = order_text(&env, &lines, "ALL", "Dubin & Sushi");
    assert!(t.contains("#abcdefgh"));
    assert!(t.contains("2 × Sake — 1500 ALL"));
    assert!(t.contains("delivery 300 ALL"));
    assert!(t.ends_with("1800 ALL · cash"));
}

/// THE KITCHEN'S FIRST LINE IS WHERE IT GOES, and this is the one that
/// used to be wrong for a whole kind. The branch was `if kind == "pickup"
/// { 🥡 } else { 🛵 }`, so an order placed at a table printed "delivery"
/// and sent somebody looking for an address that does not exist.
#[test]
fn a_table_order_says_which_table_and_never_says_delivery() {
    let env = json!({ "id": "abcdefghijkl", "contact": { "name": "Ana", "phone": "+355" },
        "fulfilment": { "kind": "dine_in", "table": "7" },
        "payment": "cash", "total": 1500 });
    let lines = [LineOut { name: "Sake".into(), quantity: 2, unit_price: 750 }];
    let t = order_text(&env, &lines, "ALL", "Dubin & Sushi");
    assert!(t.contains("table 7"), "the ticket must name the table: {t}");
    assert!(!t.contains("delivery"), "and must not call it a delivery: {t}");
}

/// A pickup is still a pickup. Asserted beside the above because the fix
/// for one kind is exactly how the other two get broken.
#[test]
fn a_pickup_is_unchanged_by_the_third_kind_arriving() {
    let env = json!({ "id": "abcdefghijkl", "contact": { "name": "Ana", "phone": "+355" },
        "fulfilment": { "kind": "pickup" }, "payment": "cash", "total": 1500 });
    let t = order_text(&env, &[], "ALL", "Dubin & Sushi");
    assert!(t.contains("pickup"), "{t}");
    assert!(!t.contains("table"), "{t}");
}

fn ticket(kind: &str) -> String {
    let env = json!({ "id": "abcdefghijkl",
        "contact": { "name": "Arben Hoxha Kola", "phone": "+355 69 123 4567" },
        "fulfilment": { "kind": kind, "table": "4", "address": { "line": "Rruga Taulantia 12" } },
        "payment": "cash", "total": 1500 });
    order_text(&env, &[LineOut { name: "Maki".into(), quantity: 1, unit_price: 1500 }], "ALL", "Dubin")
}

fn digits_run(s: &str) -> usize {
    let (mut best, mut run) = (0, 0);
    for c in s.chars() {
        run = if c.is_ascii_digit() || (run > 0 && c == ' ') { run + c.is_ascii_digit() as usize } else { 0 };
        best = best.max(run);
    }
    best
}

/// P11's CHECK: a pickup ticket names the person to call out and carries no
/// number; the same for a table. Its twin: a delivery's ticket DOES carry the
/// phone, because the door may have to be rung.
#[test]
fn a_pickup_ticket_has_no_phone_and_a_delivery_ticket_has_one() {
    for kind in ["pickup", "dine_in"] {
        let t = ticket(kind);
        assert!(!t.contains("123 4567") && !t.contains("+355"), "{kind}: {t}");
        assert!(digits_run(&t) < 7, "{kind} carries a phone-length digit run: {t}");
        assert!(t.contains("Arben K."), "{kind}: the name it is called out by: {t}");
    }
    let t = ticket("delivery");
    assert!(t.contains("+355 69 123 4567"), "a delivery keeps the phone: {t}");
    assert!(t.contains("Rruga Taulantia 12"), "and the address: {t}");
}

/// The whole name never reaches the chat: first name and one initial.
#[test]
fn the_name_on_a_ticket_is_the_first_name_and_an_initial() {
    assert_eq!(ticket_contact("Arben Hoxha Kola", "", "pickup").as_deref(), Some("Arben K."));
    assert_eq!(ticket_contact("  Ana  ", "", "pickup").as_deref(), Some("Ana"));
    assert_eq!(ticket_contact("", "+355691234567", "pickup"), None);
    assert_eq!(ticket_contact("", "+355691234567", "delivery").as_deref(), Some("+355691234567"));
    assert!(!ticket("pickup").contains("Hoxha"));
}
