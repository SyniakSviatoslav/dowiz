//! The fold, pinned against what it gets wrong. The mask that goes with it is
//! `dowiz_hub::redact`, tested where it lives.

use super::roll::*;
use serde_json::json;

// ── the fold ────────────────────────────────────────────────────────────────

fn order(phone: &str, name: &str, at: i64, total: i64, tip: i64, status: &str) -> serde_json::Value {
    json!({
        "contact": { "phone": phone, "name": name },
        "created_at_ms": at, "total": total, "tip": tip, "status": status
    })
}

fn rolled(orders: &[serde_json::Value], sort: Sort) -> Vec<Row> {
    roll(orders, |p| format!("k:{p}"), |n| n.to_string(), |p| p.to_string(), sort)
}

/// A REFUSED ORDER IS NOT MONEY TAKEN, AND THE TIP WENT TO THE COURIER. A
/// "spent" column that counted either would rank a generous customer, or one
/// the venue kept turning away, above a profitable one.
#[test]
fn a_refused_order_is_a_visit_without_money_and_the_tip_is_never_the_venues() {
    let os = [
        order("1", "A", 10, 1000, 200, "DELIVERED"),
        order("1", "A", 20, 5000, 0, "REJECTED"),
        order("1", "A", 30, 4000, 500, "CANCELLED"),
    ];
    let r = rolled(&os, Sort::Recent);
    assert_eq!(r.len(), 1, "one person");
    assert_eq!(r[0].orders, 3, "three visits, and the venue refused two of them");
    assert_eq!(r[0].spent, 800, "1000 less the 200 tip; the refused orders are nothing");
    assert_eq!(r[0].last_at, 30);
}

/// THE ROW IS THE PERSON, not the order. The key is what joins them, so two
/// spellings that hash alike are one row and the name shown is the one from
/// the order the venue saw first.
#[test]
fn one_person_is_one_row_however_many_orders_they_placed() {
    let os = [
        order("+355 69 1", "Arben", 10, 100, 0, "DELIVERED"),
        order("+355 69 1", "A.", 20, 200, 0, "DELIVERED"),
    ];
    let r = rolled(&os, Sort::Recent);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "Arben", "the first name seen, not the latest");
    assert_eq!(r[0].spent, 300);
}

/// AN ORDER WITHOUT A PHONE IS NOT A PERSON. The number is optional by
/// operator decision, so such an order is simply not attributable — and it
/// must not become a row keyed on an empty string, which would gather every
/// anonymous order in the venue's history into one fictitious customer.
#[test]
fn an_order_with_no_phone_is_skipped_and_never_becomes_one_fictitious_customer() {
    let os = [
        json!({ "contact": { "name": "A" }, "created_at_ms": 10, "total": 100, "status": "DELIVERED" }),
        json!({ "created_at_ms": 20, "total": 100, "status": "DELIVERED" }),
        order("1", "B", 30, 100, 0, "DELIVERED"),
    ];
    let r = rolled(&os, Sort::Recent);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].name, "B");
}

/// TIES BREAK ON RECENCY, or the list wobbles between two requests that asked
/// the same question.
#[test]
fn every_sort_breaks_its_ties_on_who_was_here_last() {
    let os = [
        order("old", "O", 10, 1000, 0, "DELIVERED"),
        order("new", "N", 99, 1000, 0, "DELIVERED"),
    ];
    for sort in [Sort::Spent, Sort::Orders] {
        let r = rolled(&os, sort);
        assert_eq!(r[0].name, "N", "{sort:?}: equal on the measure, later in time");
    }
    assert_eq!(rolled(&os, Sort::Recent)[0].name, "N");
}

/// THE SORTS ACTUALLY SORT, and they are different questions.
#[test]
fn spent_and_orders_are_two_different_orderings() {
    let os = [
        order("big", "B", 10, 9000, 0, "DELIVERED"),
        order("often", "F", 20, 100, 0, "DELIVERED"),
        order("often", "F", 30, 100, 0, "DELIVERED"),
        order("often", "F", 40, 100, 0, "DELIVERED"),
    ];
    assert_eq!(rolled(&os, Sort::Spent)[0].name, "B");
    assert_eq!(rolled(&os, Sort::Orders)[0].name, "F");
    assert_eq!(rolled(&os, Sort::Recent)[0].name, "F", "newest first by default");
}

/// AN UNKNOWN `sort=` IS THE DEFAULT, not an error and not an empty list: a
/// console with a stale query string still answers the useful question.
#[test]
fn an_unknown_sort_falls_back_to_newest_first() {
    assert_eq!(Sort::of(None), Sort::Recent);
    assert_eq!(Sort::of(Some("")), Sort::Recent);
    assert_eq!(Sort::of(Some("SPENT")), Sort::Recent, "and it is case-sensitive");
    assert_eq!(Sort::of(Some("spent")), Sort::Spent);
    assert_eq!(Sort::of(Some("orders")), Sort::Orders);
}
