//! The tab's numbers over envelopes in the shape `map::to_order` writes.

use super::{month, DAYS};
use crate::hubdo::OrderView;
use dowiz_hub::tz::DEFAULT;
use serde_json::{json, Value};

/// 2026-11-03T18:00:00Z: a Tuesday, 19:00 in Europe/Tirane (winter time).
const NOW: i64 = 1_793_728_800_000;
const H: i64 = 3_600_000;

fn view(o: Value) -> OrderView {
    OrderView { order_id: o["id"].as_str().unwrap().to_string(), kind: 0, seq: o["created_at_ms"].as_i64().unwrap() as u64, order_json: o.to_string() }
}

fn till(uuid: &str, at: i64, table: Option<&str>, pay: &str, items: Vec<Value>, status: &str) -> Value {
    let total: i64 = items.iter().map(|i| i["quantity"].as_i64().unwrap() * i["unit_price"].as_i64().unwrap()).sum();
    let mut o = json!({
        "id": format!("ebills:{uuid}"), "status": status, "channel": "ebills", "items": items, "subtotal": total, "discount": 0,
        "total": total, "delivery_fee": 0, "tip": 0, "created_at_ms": at, "location_id": "loc", "currency": "ALL", "payment": pay,
        "fulfilment": match table { Some(t) => json!({ "kind": "dine_in", "table": t, "fee": 0 }), None => json!({ "kind": "pickup", "code": "", "fee": 0 }) },
        "external": { "source": "ebills", "sale_id": 9000 + at % 1000, "inv_ord_num": 7, "uuid": uuid, "fic": "FIC", "operator_id": 3, "log_cis_len": 1 },
    });
    if table.is_none() {
        o["payment_status"] = json!("paid");
    }
    o
}

fn mapped(name: &str, code: &str, pid: &str, how: &str, q: i64, p: i64) -> Value {
    json!({ "product_id": pid, "ebills_code": code, "matched_by": how, "name": name, "quantity": q, "unit_price": p, "modifier_ids": [], "discount_pct": 0 })
}

fn raw(name: &str, code: &str, q: i64, p: i64) -> Value {
    json!({ "product_id": format!("ebills:{code}"), "name": name, "quantity": q, "unit_price": p, "modifier_ids": [], "discount_pct": 0 })
}

fn venue() -> Vec<OrderView> {
    vec![
        // Today 19:00 local: a course at table 4, two lines, one matched by the owner, one by name.
        view(till("a", NOW, Some("4"), "cash", vec![mapped("Sake roll", "56", "p-roll", "map", 2, 250), mapped("Miso", "12", "p-miso", "name", 1, 200)], "PICKED_UP")),
        // Today 12:30 local: a counter sale by card, unmatched line.
        view(till("b", NOW - 6 * H - 30 * 60_000, None, "card", vec![raw("Dragon roll", "77", 1, 900)], "PICKED_UP")),
        // Yesterday: a course whose sale was later voided (refunded): a visit, no money, no dishes.
        view(till("c", NOW - 25 * H, Some("2"), "cash", vec![raw("Dragon roll", "77", 3, 900)], "COMPENSATED_REFUND")),
        // 29 days ago, inside the window; 31 days ago, outside it.
        view(till("d", NOW - 29 * 24 * H, Some("1"), "cash", vec![raw("Edamame", "5", 1, 150)], "PICKED_UP")),
        view(till("e", NOW - 31 * 24 * H, Some("1"), "cash", vec![raw("Edamame", "5", 1, 150)], "PICKED_UP")),
        // A storefront delivery the same day: not the till's.
        view(json!({ "id": "o1", "status": "DELIVERED", "items": [raw("Edamame", "5", 4, 150)], "total": 600, "tip": 50, "created_at_ms": NOW - H, "fulfilment": { "kind": "delivery" } })),
        // Dated in the future: not yet today's.
        view(till("f", NOW + H, None, "cash", vec![raw("Edamame", "5", 1, 150)], "PICKED_UP")),
    ]
}

#[test]
fn only_the_tills_orders_of_the_window_newest_first() {
    let v = month(&venue(), DEFAULT, NOW);
    let ids: Vec<&str> = v["orders"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["ebills:a", "ebills:b", "ebills:c", "ebills:d"]);
    assert_eq!(v["days"], json!(DAYS));
    assert_eq!(v["by_day"].as_array().unwrap().len(), DAYS as usize);
    assert_eq!(v["by_day"][DAYS as usize - 1]["day"], json!("2026-11-03"));
    assert_eq!(v["by_day"][0]["day"], json!("2026-10-05"));
}

#[test]
fn the_money_is_venue_took_and_the_void_is_a_visit_without_money() {
    let v = month(&venue(), DEFAULT, NOW);
    assert_eq!((v["count"].as_i64(), v["counted"].as_i64(), v["not_taken"].as_i64()), (Some(4), Some(3), Some(1)));
    assert_eq!(v["revenue"], json!(700 + 900 + 150));
    assert_eq!(v["average_check"], json!(1750 / 3));
    assert_eq!(v["by_payment"]["cash"], json!({ "orders": 3, "revenue": 850 }));
    assert_eq!(v["by_payment"]["card"], json!({ "orders": 1, "revenue": 900 }));
    assert_eq!(v["by_kind"], json!({ "dine_in": 3, "pickup": 1 }));
    // Two courses in takings are unpaid by a bill yet; the voided one is not counted.
    assert_eq!(v["unpaid_courses"], json!(2));
}

#[test]
fn hours_and_weekdays_are_the_venues_not_utcs() {
    let v = month(&venue(), DEFAULT, NOW);
    // 18:00Z is 19:00 in Tirane; 11:30Z is 12:30.
    assert_eq!(v["by_hour"][19], json!({ "orders": 1, "revenue": 700 }));
    assert_eq!(v["by_hour"][12], json!({ "orders": 1, "revenue": 900 }));
    // The void was 17:00Z = 18:00 local on Monday: a visit at that hour, no money.
    assert_eq!(v["by_hour"][18], json!({ "orders": 1, "revenue": 0 }));
    // Tuesday is index 1 (Monday 0); the void was Monday 18:00 local.
    assert_eq!(v["by_weekday"][1]["orders"], json!(2));
    // Monday: the void (yesterday) and the course 29 days ago (Oct 5, a Monday).
    assert_eq!(v["by_weekday"][0], json!({ "orders": 2, "revenue": 150 }));
    assert_eq!(v["by_day"][DAYS as usize - 1], json!({ "day": "2026-11-03", "at_ms": NOW - 19 * H, "orders": 2, "revenue": 1600 }));
}

#[test]
fn lines_are_split_by_how_they_were_matched_and_the_unmatched_are_named() {
    let v = month(&venue(), DEFAULT, NOW);
    assert_eq!(v["matched"]["map"], json!({ "lines": 1, "quantity": 2, "revenue": 500 }));
    assert_eq!(v["matched"]["name"], json!({ "lines": 1, "quantity": 1, "revenue": 200 }));
    // The voided sale's Dragon rolls are not sold; the counter sale's one is.
    assert_eq!(v["sold"], json!(500 + 200 + 900 + 150));
    assert_eq!((v["unmatched"]["lines"].as_i64(), v["unmatched"]["quantity"].as_i64(), v["unmatched"]["revenue"].as_i64()), (Some(2), Some(2), Some(1050)));
    assert_eq!(v["unmatched"]["share_ppm"], json!(1050 * 1_000_000 / 1750));
    let names: Vec<(&str, &str)> = v["unmatched"]["names"].as_array().unwrap().iter().map(|u| (u["name"].as_str().unwrap(), u["code"].as_str().unwrap())).collect();
    assert_eq!(names, vec![("Dragon roll", "77"), ("Edamame", "5")]);
    assert_eq!(v["items"][0], json!({ "name": "Dragon roll", "code": "77", "product_id": null, "matched_by": null, "quantity": 1, "revenue": 900 }));
    assert_eq!(v["items"][1]["product_id"], json!("p-roll"));
    assert_eq!(v["tables"][0], json!({ "table": "4", "orders": 1, "revenue": 700 }));
    assert_eq!(v["operators"], json!([{ "operator_id": "3", "orders": 4, "revenue": 1750 }]));
}

#[test]
fn a_row_carries_the_tills_facts_and_nothing_invented() {
    let v = month(&venue(), DEFAULT, NOW);
    let r = &v["orders"][0];
    assert_eq!(r["sale_id"], json!(9000 + NOW % 1000));
    assert_eq!((r["fic"].as_str(), r["table"].as_str(), r["payment"].as_str(), r["kind"].as_str()), (Some("FIC"), Some("4"), Some("cash"), Some("dine_in")));
    assert_eq!(r["payment_status"], Value::Null);
    assert_eq!(r["items"][0], json!({ "code": "56", "name": "Sake roll", "product_id": "p-roll", "matched_by": "map", "quantity": 2, "unit_price": 250, "discount_pct": 0, "cost": null }));
    assert_eq!(v["orders"][1]["items"][0]["code"], json!("77"));
    assert_eq!(v["orders"][1]["items"][0]["product_id"], Value::Null);
    assert_eq!(v["orders"][1]["payment_status"], json!("paid"));
}

#[test]
fn an_empty_log_is_thirty_empty_days() {
    let v = month(&[], DEFAULT, NOW);
    assert_eq!((v["count"].as_i64(), v["revenue"].as_i64(), v["average_check"].as_i64()), (Some(0), Some(0), Some(0)));
    assert_eq!(v["orders"], json!([]));
    assert_eq!(v["unmatched"]["share_ppm"], json!(0));
    assert!(v["by_day"].as_array().unwrap().iter().all(|d| d["orders"] == json!(0)));
}

/// `log.placed` v2 (W-EBILLS-TAB, 2026-11-03): a matched till line gained `matched_by`.
/// Sigma: a v1 line matched without it was matched by the owner's mapping (the only way
/// then), so it reads as `"map"`; Delta o Sigma = id: a month of v1 lines reads exactly as
/// the same month written as v2 with `"map"`; and an unmatched line is unchanged by the field.
#[test]
fn placed_gained_matched_by_and_the_laws_hold() {
    let v1 = json!({ "product_id": "p-roll", "ebills_code": "56", "name": "Sake roll", "quantity": 2, "unit_price": 250, "modifier_ids": [], "discount_pct": 0 });
    let v2 = mapped("Sake roll", "56", "p-roll", "map", 2, 250);
    let old = vec![view(till("a", NOW, Some("4"), "cash", vec![v1, raw("Edamame", "5", 1, 150)], "PICKED_UP"))];
    let new = vec![view(till("a", NOW, Some("4"), "cash", vec![v2, raw("Edamame", "5", 1, 150)], "PICKED_UP"))];
    let (a, b) = (month(&old, DEFAULT, NOW), month(&new, DEFAULT, NOW));
    assert_eq!(a, b, "a v1 month reads as the v2 month with matched_by = map");
    assert_eq!(a["orders"][0]["items"][0]["matched_by"], json!("map"), "Sigma: map");
    assert_eq!(a["orders"][0]["items"][1]["matched_by"], json!(null), "an unmatched line has no matched_by in either version");
    assert_eq!(a["matched"]["map"], json!({ "lines": 1, "quantity": 2, "revenue": 500 }));
}
