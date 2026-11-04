//! The split's arithmetic, and the pane's period over hot orders and cold rows.

use super::*;
use crate::services::analytics::cube::DayCube;
use crate::services::analytics::history::Span;
use dowiz_hub::tz::Zone;

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's own zone")
}

/// 2026-11-03T18:00:00Z: 19:00 in Tirane; day number of 2026-11-03.
const NOW: i64 = 1_793_728_800_000;
const H: i64 = 3_600_000;
const TODAY: i64 = 20_760;

fn sides() -> Sides {
    Sides::with(&[("p-set", Side::Kitchen), ("p-roll", Side::Kitchen), ("p-cola", Side::Drinks)])
}

fn order(at: i64, status: &str, items: Vec<Value>) -> Value {
    let total: i64 = items.iter().map(|i| i["quantity"].as_i64().unwrap() * i["unit_price"].as_i64().unwrap()).sum();
    json!({ "created_at_ms": at, "status": status, "total": total, "tip": 0, "items": items })
}

fn it(pid: &str, name: &str, q: i64, p: i64) -> Value {
    json!({ "product_id": pid, "name": name, "quantity": q, "unit_price": p })
}

#[test]
fn lines_and_checks_add_up_per_side_and_the_shares_are_per_mille_of_the_lines() {
    let mut sp = Split::default();
    sp.add_order(&order(NOW, "PICKED_UP", vec![it("p-set", "Set", 1, 2800), it("p-cola", "cola", 2, 100)]), "2026-11-03", &sides());
    sp.add_order(&order(NOW - H, "DELIVERED", vec![it("p-cola", "cola", 1, 100)]), "2026-11-03", &sides());
    sp.add_order(&order(NOW - 2 * H, "PICKED_UP", vec![it("ebills:1", "ruby no hitomi", 1, 500)]), "2026-11-03", &sides());
    // Refused and still waiting: no line, no check.
    sp.add_order(&order(NOW - 3 * H, "REJECTED", vec![it("p-set", "Set", 1, 2800)]), "2026-11-03", &sides());
    sp.add_order(&order(NOW - 3 * H, "PENDING", vec![it("p-set", "Set", 1, 2800)]), "2026-11-03", &sides());
    let v = sp.json(&["2026-11-02".into(), "2026-11-03".into()], "2026-11-02", "2026-11-03");
    assert_eq!(v["sold"], json!(2800 + 300 + 500));
    assert_eq!((v["kitchen"]["quantity"].as_i64(), v["kitchen"]["sales"].as_i64(), v["kitchen"]["sharePm"].as_i64()), (Some(1), Some(2800), Some(2800 * 1000 / 3600)));
    assert_eq!((v["drinks"]["quantity"].as_i64(), v["drinks"]["sales"].as_i64()), (Some(3), Some(300)));
    assert_eq!((v["unclassified"]["quantity"].as_i64(), v["unclassified"]["sales"].as_i64()), (Some(1), Some(500)));
    assert_eq!(v["drinks"]["top"], json!([{ "id": "p-cola", "name": "cola", "quantity": 3, "sales": 300 }]), "one dish, two orders, one row");
    assert_eq!(v["orders"]["kitchen"], json!({ "orders": 1, "revenue": 3000, "averageCheck": 3000 }));
    assert_eq!(v["orders"]["drinksOnly"], json!({ "orders": 1, "revenue": 100, "averageCheck": 100 }));
    assert_eq!(v["orders"]["unclassified"], json!({ "orders": 1, "revenue": 500, "averageCheck": 500 }));
    assert_eq!(v["orders"]["from"], json!("2026-11-02"));
    assert_eq!(v["byDay"], json!([{ "day": "2026-11-02", "kitchen": 0, "drinks": 0, "unclassified": 0 }, { "day": "2026-11-03", "kitchen": 2800, "drinks": 300, "unclassified": 500 }]));
}

#[test]
fn nothing_sold_has_no_share_and_zero_averages() {
    let v = Split::default().json(&[], "2026-11-03", "2026-11-03");
    assert_eq!(v["kitchen"]["sharePm"], Value::Null, "no whole, no share");
    assert_eq!(v["orders"]["kitchen"]["averageCheck"], json!(0));
    assert_eq!(v["byDay"], json!([]));
}

#[test]
fn the_top_lists_the_dearest_first_and_at_most_top_rows() {
    let mut sp = Split::default();
    for i in 0..(TOP as i64 + 3) {
        sp.add_line(Some(Side::Kitchen), "2026-11-03", &format!("d{i}"), json!(format!("Dish {i}")), 1, 100 + i);
    }
    let v = sp.json(&[], "", "");
    let top = v["kitchen"]["top"].as_array().unwrap();
    assert_eq!(top.len(), TOP);
    assert_eq!(top[0]["id"], json!(format!("d{}", TOP as i64 + 2)));
}

/// The pane's period: checks and lines from the hot orders inside it, lines from
/// the cube's rows (by id, through the catalogue's names), and `orders.from`
/// honest about the first hot day.
#[test]
fn a_period_takes_hot_orders_and_cold_rows_and_says_where_its_checks_start() {
    let z = tirane();
    let hot = vec![
        order(NOW, "PICKED_UP", vec![it("p-set", "Set", 1, 2800), it("p-cola", "cola", 1, 100)]),
        // Dated after `now`: nobody's yet.
        order(NOW + H, "PICKED_UP", vec![it("p-set", "Set", 1, 2800)]),
        // Two days before the period: outside it, but it dates the hot log.
        order(NOW - 9 * 24 * H, "PICKED_UP", vec![it("p-cola", "cola", 1, 100)]),
    ];
    let mut cold = BTreeMap::new();
    let mut r = DayCube::new(20_261_030);
    r.m.insert("p-roll".into(), [4, 4000, 0, 0]);
    r.m.insert("ebills:56".into(), [10, 790, 0, 0]);
    r.m.insert("ebills:183".into(), [1, 500, 0, 0]);
    cold.insert(20_261_030, r);
    // A row before the period: not read.
    let mut before = DayCube::new(20_261_020);
    before.m.insert("p-roll".into(), [9, 9000, 0, 0]);
    cold.insert(20_261_020, before);
    let name = |id: &str| json!(match id { "p-roll" => "Sake roll", "ebills:56" => "kafe", "p-set" => "Set", "p-cola" => "cola", other => other });
    let s = Span { first: TODAY - 6, n: 7 };
    let v = period(&hot, &cold, z, s, NOW, &sides(), &name);
    assert_eq!((v["kitchen"]["quantity"].as_i64(), v["kitchen"]["sales"].as_i64()), (Some(5), Some(2800 + 4000)), "the hot set and the archived rolls");
    assert_eq!((v["drinks"]["quantity"].as_i64(), v["drinks"]["sales"].as_i64()), (Some(11), Some(100 + 790)), "the hot cola and the archived coffees, by the word on the archived line's name");
    assert_eq!((v["unclassified"]["quantity"].as_i64(), v["unclassified"]["sales"].as_i64()), (Some(1), Some(500)), "an archived till line with no name left is unclassified");
    assert_eq!(v["orders"]["kitchen"], json!({ "orders": 1, "revenue": 2900, "averageCheck": 2900 }));
    assert_eq!(v["orders"]["from"], json!("2026-10-28"), "the period starts before the hot log's first day? no: the hot log is older, so the period's first day");
    assert_eq!(v["orders"]["to"], json!("2026-11-03"));
    assert_eq!(v["byDay"].as_array().unwrap().len(), 7);
    assert_eq!(v["byDay"][2], json!({ "day": "2026-10-30", "kitchen": 4000, "drinks": 790, "unclassified": 500 }));
    assert_eq!(v["byDay"][6]["kitchen"], json!(2800));
}

/// Twin: a hot log younger than the period says so.
#[test]
fn a_young_hot_log_moves_the_checks_start_forward() {
    let hot = vec![order(NOW, "PICKED_UP", vec![it("p-set", "Set", 1, 2800)])];
    let s = Span { first: TODAY - 29, n: 30 };
    let v = period(&hot, &BTreeMap::new(), tirane(), s, NOW, &sides(), &|id: &str| json!(id));
    assert_eq!(v["orders"]["from"], json!("2026-11-03"));
    assert_eq!(v["orders"]["to"], json!("2026-11-03"));
}
