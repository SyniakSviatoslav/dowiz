//! The week's badge, pinned: its window, its exclusions, its threshold, and
//! that the owner's pane and the public badge are one fold.

use super::*;
use serde_json::json;

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's zone")
}
/// 2026-09-22 12:00 UTC = 14:00 in Tirane (a Tuesday).
const NOON: i64 = 1_790_078_400_000;
const DAY: i64 = 86_400_000;

fn order(at: i64, status: &str, who: &str, dish: &str, q: i64) -> Value {
    json!({ "created_at_ms": at, "status": status, "contact": { "name": who, "phone": "+355690000000" },
            "items": [{ "product_id": dish, "quantity": q, "unit_price": 500 }] })
}
fn n_of(all: &[Count], id: &str) -> (i64, i64) {
    all.iter().find(|c| c.id == id).map_or((0, 0), |c| (c.n, c.test))
}

#[test]
fn the_window_is_seven_local_days_today_included() {
    assert_eq!(window(tirane(), NOON), (20260916, 20260922));
    let os = [
        order(NOON, "DELIVERED", "Ana", "maki", 2),
        order(NOON - 6 * DAY, "DELIVERED", "Ana", "maki", 3),  // 09-16: the first day, in
        order(NOON - 7 * DAY, "DELIVERED", "Ana", "maki", 50), // 09-15: out
        order(NOON + DAY, "DELIVERED", "Ana", "maki", 50),     // tomorrow: nobody's day yet
    ];
    assert_eq!(n_of(&counts(&os, tirane(), NOON), "maki"), (5, 0));
}

#[test]
fn refused_and_refunded_orders_count_nowhere() {
    let os = [
        order(NOON, "DELIVERED", "Ana", "maki", 1),
        order(NOON, "REJECTED", "Ana", "maki", 10),
        order(NOON, "CANCELLED", "Ana", "maki", 10),
        order(NOON, "COMPENSATED_REFUND", "Ana", "maki", 10),
        order(NOON, "PENDING", "Ana", "maki", 1),
    ];
    assert_eq!(n_of(&counts(&os, tirane(), NOON), "maki"), (2, 0), "taken orders only, as the pane counts");
}

#[test]
fn a_test_order_is_seen_and_left_out() {
    let os = [
        order(NOON, "DELIVERED", "Ana", "maki", 1),
        order(NOON, "DELIVERED", "LIVE-mg3x guest", "maki", 4),
        order(NOON, "CONFIRMED", "FLOWS-abc", "maki", 3),
        order(NOON, "DELIVERED", "Lived here", "maki", 1), // a real name that merely starts with "Live"
    ];
    assert_eq!(n_of(&counts(&os, tirane(), NOON), "maki"), (2, 7));
    assert!(is_test(&os[1]) && is_test(&os[2]) && !is_test(&os[3]));
}

#[test]
fn the_badge_needs_the_threshold_and_shows_the_number() {
    let mut os: Vec<Value> = (0..THRESHOLD - 1).map(|i| order(NOON - i * 3_600_000, "DELIVERED", "Ana", "maki", 1)).collect();
    os.push(order(NOON, "DELIVERED", "LIVE-x guest", "maki", 100));
    let p = public(&os, tirane(), NOON);
    assert_eq!(p["dishes"], json!([]), "one short of the threshold, and test plates never lift it");
    os.push(order(NOON, "DELIVERED", "Ana", "maki", 1));
    let p = public(&os, tirane(), NOON);
    assert_eq!(p["dishes"], json!([{ "id": "maki", "n": THRESHOLD }]));
    assert_eq!(p["contract"], CONTRACT);
    assert!(p.to_string().find("test").is_none(), "the public answer carries no test count: {p}");
}

#[test]
fn at_most_max_badges_most_ordered_first_then_by_id() {
    let os: Vec<Value> = (0..MAX_BADGES as i64 + 2)
        .map(|i| order(NOON, "DELIVERED", "Ana", &format!("d{i}"), THRESHOLD + (i % 3)))
        .collect();
    let p = public(&os, tirane(), NOON);
    let ids: Vec<&str> = p["dishes"].as_array().unwrap().iter().map(|d| d["id"].as_str().unwrap()).collect();
    assert_eq!(ids.len(), MAX_BADGES);
    assert_eq!(&ids[..3], ["d2", "d5", "d1"], "{ids:?}");
}

/// THE PANE AND THE BADGE ARE ONE FOLD: every badged dish's public n is the
/// owner's n for the same dish over the same window.
#[test]
fn the_owner_pane_shows_the_same_number_as_the_badge() {
    let mut os: Vec<Value> = (0..9).map(|i| order(NOON - i * DAY / 2, "DELIVERED", "Ana", "maki", 1)).collect();
    os.push(order(NOON, "DELIVERED", "LIVE-r guest", "maki", 2));
    os.push(order(NOON, "DELIVERED", "Ana", "cola", 1));
    let p = public(&os, tirane(), NOON);
    let o = owner(&os, tirane(), NOON, &|id| json!(format!("name of {id}")));
    assert_eq!((p["from"].clone(), p["to"].clone()), (o["from"].clone(), o["to"].clone()));
    let pub_maki = &p["dishes"][0];
    let own_maki = o["dishes"].as_array().unwrap().iter().find(|d| d["id"] == "maki").unwrap();
    assert_eq!(pub_maki["n"], own_maki["n"]);
    assert_eq!(own_maki["test"], 2);
    assert_eq!(own_maki["badge"], true);
    let cola = o["dishes"].as_array().unwrap().iter().find(|d| d["id"] == "cola").unwrap();
    assert_eq!((cola["n"].as_i64(), cola["badge"].as_bool()), (Some(1), Some(false)), "listed for the owner, no badge");
}
