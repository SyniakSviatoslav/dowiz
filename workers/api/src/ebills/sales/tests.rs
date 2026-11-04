//! The tab's numbers over envelopes in the shape `map::to_order` writes.

use super::{month, DAYS};
use crate::hubdo::OrderView;
use crate::services::analytics::side::{Side, Sides};
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
    let v = month(&venue(), DEFAULT, NOW, &Sides::default());
    let ids: Vec<&str> = v["orders"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["ebills:a", "ebills:b", "ebills:c", "ebills:d"]);
    assert_eq!(v["days"], json!(DAYS));
    assert_eq!(v["by_day"].as_array().unwrap().len(), DAYS as usize);
    assert_eq!(v["by_day"][DAYS as usize - 1]["day"], json!("2026-11-03"));
    assert_eq!(v["by_day"][0]["day"], json!("2026-10-05"));
}

#[test]
fn the_money_is_venue_took_and_the_void_is_a_visit_without_money() {
    let v = month(&venue(), DEFAULT, NOW, &Sides::default());
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
    let v = month(&venue(), DEFAULT, NOW, &Sides::default());
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
    let v = month(&venue(), DEFAULT, NOW, &Sides::default());
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
    let v = month(&venue(), DEFAULT, NOW, &Sides::default());
    let r = &v["orders"][0];
    assert_eq!(r["sale_id"], json!(9000 + NOW % 1000));
    assert_eq!((r["fic"].as_str(), r["table"].as_str(), r["payment"].as_str(), r["kind"].as_str()), (Some("FIC"), Some("4"), Some("cash"), Some("dine_in")));
    assert_eq!(r["payment_status"], Value::Null);
    // `side` is the line's kitchen/drinks answer -- here by the word "roll", since the test's catalogue is empty -- and
    // `till_category` the till's group, absent on a line placed before it was kept (W-DRINKS-KITCHEN).
    assert_eq!(r["items"][0], json!({ "code": "56", "name": "Sake roll", "product_id": "p-roll", "matched_by": "map", "quantity": 2, "unit_price": 250, "discount_pct": 0, "cost": null, "side": "kitchen", "till_category": null }));
    assert_eq!(v["orders"][1]["items"][0]["code"], json!("77"));
    assert_eq!(v["orders"][1]["items"][0]["product_id"], Value::Null);
    assert_eq!(v["orders"][1]["payment_status"], json!("paid"));
}

#[test]
fn an_empty_log_is_thirty_empty_days() {
    let v = month(&[], DEFAULT, NOW, &Sides::default());
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
    let (a, b) = (month(&old, DEFAULT, NOW, &Sides::default()), month(&new, DEFAULT, NOW, &Sides::default()));
    assert_eq!(a, b, "a v1 month reads as the v2 month with matched_by = map");
    assert_eq!(a["orders"][0]["items"][0]["matched_by"], json!("map"), "Sigma: map");
    assert_eq!(a["orders"][0]["items"][1]["matched_by"], json!(null), "an unmatched line has no matched_by in either version");
    assert_eq!(a["matched"]["map"], json!({ "lines": 1, "quantity": 2, "revenue": 500 }));
}

// ── kitchen / drinks (W-DRINKS-KITCHEN) ─────────────────────────────────────

fn till_line(name: &str, code: &str, group: &str, q: i64, p: i64) -> Value {
    json!({ "product_id": format!("ebills:{code}"), "name": name, "quantity": q, "unit_price": p, "modifier_ids": [], "discount_pct": 0, "ebills_category": group })
}

/// Two sushi tables and five coffees: one average check says 1,059; the split says 3,000 and 79.
fn bar_and_kitchen() -> Vec<OrderView> {
    let mut v = vec![
        view(till("t1", NOW, Some("4"), "cash", vec![mapped("Set Philadelphia", "10", "p-set", "name", 1, 2800), till_line("kafe", "56", "kafeteria", 2, 100)], "PICKED_UP")),
        view(till("t2", NOW - H, Some("5"), "card", vec![mapped("Sake roll", "11", "p-roll", "map", 2, 1000), raw("hot sake", "89", 1, 1000)], "PICKED_UP")),
        // Voided: a visit, no check, no line.
        view(till("v", NOW - 2 * H, Some("5"), "card", vec![mapped("Sake roll", "11", "p-roll", "map", 2, 1000)], "COMPENSATED_REFUND")),
    ];
    for i in 0..5 {
        v.push(view(till(&format!("c{i}"), NOW - 3 * H - i * 60_000, None, "cash", vec![till_line("kafe", "56", "kafeteria", 1, 79)], "PICKED_UP")));
    }
    // Yesterday: a line imported before the till's group was kept, and no dish of its name.
    v.push(view(till("u", NOW - 25 * H, None, "cash", vec![raw("ruby no hitomi", "183", 1, 500)], "PICKED_UP")));
    v
}

#[test]
fn the_tab_splits_lines_and_checks_into_kitchen_drinks_and_unclassified() {
    let sides = Sides::with(&[("p-set", Side::Kitchen), ("p-roll", Side::Kitchen)]);
    let v = month(&bar_and_kitchen(), DEFAULT, NOW, &sides);
    let sp = &v["sides"];
    // Lines: the two sets and rolls are kitchen; coffees are drinks by the till's group; "hot sake" and "ruby no hitomi" by neither.
    assert_eq!((sp["kitchen"]["quantity"].as_i64(), sp["kitchen"]["sales"].as_i64()), (Some(3), Some(2800 + 2000)));
    assert_eq!((sp["drinks"]["quantity"].as_i64(), sp["drinks"]["sales"].as_i64()), (Some(7), Some(200 + 5 * 79)));
    assert_eq!((sp["unclassified"]["quantity"].as_i64(), sp["unclassified"]["sales"].as_i64()), (Some(2), Some(1500)));
    assert_eq!(sp["sold"], v["sold"], "the sides add up to what the tab sold");
    assert_eq!(sp["kitchen"]["sharePm"], json!(4800 * 1000 / (4800 + 595 + 1500)));
    assert_eq!(sp["unclassified"]["top"][0]["name"], json!("hot sake"), "the dearer unknown line first, by name since it has no dish");
    // Checks: the two tables are kitchen checks (coffee at one of them included); the five coffees are drinks only;
    // the pre-group line alone is unclassified; the void is no check.
    assert_eq!(sp["orders"]["kitchen"], json!({ "orders": 2, "revenue": 3000 + 3000, "averageCheck": 3000 }));
    assert_eq!(sp["orders"]["drinksOnly"], json!({ "orders": 5, "revenue": 395, "averageCheck": 79 }));
    assert_eq!(sp["orders"]["unclassified"], json!({ "orders": 1, "revenue": 500, "averageCheck": 500 }));
    assert_eq!(sp["orders"]["from"], v["by_day"][0]["day"], "the tab's checks cover its whole window");
    assert_eq!(sp["orders"]["to"], json!("2026-11-03"));
    // By day: today's and yesterday's sales per side, thirty rows.
    let by_day = sp["byDay"].as_array().unwrap();
    assert_eq!(by_day.len(), DAYS as usize);
    assert_eq!(by_day[DAYS as usize - 1], json!({ "day": "2026-11-03", "kitchen": 4800, "drinks": 595, "unclassified": 1000 }));
    assert_eq!(by_day[DAYS as usize - 2], json!({ "day": "2026-11-02", "kitchen": 0, "drinks": 0, "unclassified": 500 }));
    // Each row's line says its side and the till's group.
    let t1 = v["orders"].as_array().unwrap().iter().find(|o| o["id"] == json!("ebills:t1")).unwrap();
    assert_eq!(t1["items"][0]["side"], json!("kitchen"));
    assert_eq!((t1["items"][1]["side"].clone(), t1["items"][1]["till_category"].clone()), (json!("drinks"), json!("kafeteria")));
}

/// THE LAW FOR `ebills_category` (W-DRINKS-KITCHEN, 2026-11-04): a line placed before it reads
/// by its name -- Sigma supplies the side the words give, or none -- and a month of such lines
/// reads exactly as the same month written with the group the words agree with; an unknown
/// name is unclassified, never a side by a guess.
#[test]
fn a_line_without_the_tills_group_reads_by_its_name_or_stays_unclassified() {
    let old = vec![view(till("a", NOW, None, "cash", vec![raw("kafe", "56", 1, 79), raw("ruby no hitomi", "183", 1, 500)], "PICKED_UP"))];
    let new = vec![view(till("a", NOW, None, "cash", vec![till_line("kafe", "56", "kafeteria", 1, 79), raw("ruby no hitomi", "183", 1, 500)], "PICKED_UP"))];
    let (a, b) = (month(&old, DEFAULT, NOW, &Sides::default()), month(&new, DEFAULT, NOW, &Sides::default()));
    assert_eq!(a["sides"], b["sides"], "Delta o Sigma = id on the numbers");
    assert_eq!(a["orders"][0]["items"][0]["side"], json!("drinks"), "Sigma: by the word");
    assert_eq!(a["orders"][0]["items"][0]["till_category"], json!(null), "and no group is invented");
    assert_eq!(b["orders"][0]["items"][0]["till_category"], json!("kafeteria"));
    assert_eq!(a["orders"][0]["items"][1]["side"], json!(null), "unknown stays unknown");
    assert_eq!(a["sides"]["orders"]["unclassified"]["orders"], json!(1), "a check with an unknown line and no kitchen line is unclassified");
}
