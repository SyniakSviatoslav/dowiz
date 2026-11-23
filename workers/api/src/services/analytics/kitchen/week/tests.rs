//! The week's purchase list over a real stock log and real order JSON: the
//! average stands on the days draws cover, the stock is subtracted only where
//! it was counted and is not a minus, packs round up, an unknown price stays
//! unknown, and the share of sales no recipe draws for is said.

use super::*;
use crate::services::analytics::kitchen::sales::{self, DishLine};
use crate::services::operations::stock::suppliers::Card;
use dowiz_hub::stock::meta::{day_number, day_of_number, Meta};
use dowiz_hub::stock::{StockEvent, StockLog, WasteReason};

const DAY: i64 = 86_400_000;

/// Four whole weeks, 2026-10-26 .. 2026-11-22.
fn weeks() -> Window {
    let first = day_number(20261026);
    Window { starts: (0..28).map(|k| k * DAY).collect(), end: 28 * DAY, days: (0..28).map(|k| day_of_number(first + k)).collect() }
}

fn sup(id: &str, list: Option<i64>) -> Supply {
    Supply { id: id.into(), name: id.to_uppercase(), unit: "g".into(), basis: 100, list_cost: list, clean_pm: 1000, cook_pm: 1000 }
}

fn sell(log: &mut StockLog, item: &str, qty: i64, order: &str, at: i64) {
    log.set_clock(at);
    log.append_all(&[StockEvent::Reserved { item: item.into(), qty, order_id: order.into() }]).unwrap();
    log.append_all(&[StockEvent::Consumed { item: item.into(), qty, order_id: order.into() }]).unwrap();
}

fn buy(lead: i64, pack: Option<i64>) -> Buy {
    Buy { lead_days: lead, pack, supplier: String::new() }
}

/// Draws began on day 14: the average stands on 14 days, not 28 -- the two
/// weeks before are no weeks of zero use, and a write-off in them is not use
/// of the weeks measured. Counted rice: (weekly x (7 + 2 lead) / 7) - free,
/// up to a pack of 1000. Salmon below zero and nori never counted: the
/// weekly need, and no number pretending to be "to buy".
#[test]
fn the_week_stands_on_the_days_the_ledger_drew() {
    let w = weeks();
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(DAY);
    log.receive_with("salmon", 1000, &Meta::default()).unwrap();
    log.receive_with("rice", 5000, &Meta::default()).unwrap();
    log.set_clock(3 * DAY);
    log.append(&StockEvent::Wasted { item: "rice".into(), qty: 999, reason: WasteReason::Spoiled, by: "p".into() }).unwrap();
    sell(&mut log, "salmon", 700, "o1", 14 * DAY + 5);
    // The till's sale is not refused past the shelf (eBills): the counted salmon goes below zero.
    log.set_clock(20 * DAY);
    log.append(&StockEvent::Served { item: "salmon".into(), qty: 700, order_id: "till-0".into() }).unwrap();
    sell(&mut log, "rice", 3201, "o3", 15 * DAY);
    // A till sale (eBills) draws through the same ledger.
    log.set_clock(27 * DAY);
    log.append(&StockEvent::Served { item: "nori".into(), qty: 140, order_id: "till-1".into() }).unwrap();
    let j = log.journal().unwrap();
    let supplies = HashMap::from([("salmon".to_string(), sup("salmon", None)), ("rice".to_string(), sup("rice", Some(50))), ("nori".to_string(), sup("nori", None))]);
    let buys = HashMap::from([("salmon".to_string(), buy(1, None)), ("rice".to_string(), buy(2, Some(1000))), ("nori".to_string(), buy(1, None))]);
    let wk = block(&sales::Sales::default(), &j, &HashMap::new(), &supplies, &buys, &w);

    assert_eq!((wk["from"].clone(), wk["to"].clone(), wk["dataFrom"].clone(), wk["dataDays"].clone()), (json!("2026-10-26"), json!("2026-11-22"), json!("2026-11-09"), json!(14)));
    let row = |id: &str| wk["rows"].as_array().unwrap().iter().find(|r| r["id"] == id).cloned().unwrap();
    let rice = row("rice");
    // 3201 over 14 days: 1601 a week (up); need 3201 x 9 / 14 = 2058 (up); 800 free -> 1258 -> two packs of 1000.
    assert_eq!((rice["used"].clone(), rice["weekly"].clone(), rice["need"].clone(), rice["leadDays"].clone()), (json!(3201), json!(1601), json!(2058), json!(2)));
    assert_eq!((rice["available"].clone(), rice["stockKnown"].clone(), rice["toBuy"].clone()), (json!(800), json!(true), json!(2000)));
    // 2000 g at 50 per 100 g.
    assert_eq!((rice["cost"].clone(), rice["priced"].clone()), (json!(1000), json!(true)));
    let salmon = row("salmon");
    assert_eq!((salmon["weekly"].clone(), salmon["need"].clone(), salmon["available"].clone()), (json!(700), json!(800), json!(-400)));
    assert_eq!((salmon["stockKnown"].clone(), salmon["toBuy"].clone()), (json!(false), Value::Null), "a minus is not subtracted, and no number pretends");
    assert_eq!((salmon["cost"].clone(), salmon["priced"].clone()), (Value::Null, json!(false)), "no price is no price, never 0");
    let nori = row("nori");
    assert_eq!((nori["weekly"].clone(), nori["need"].clone(), nori["stockKnown"].clone(), nori["toBuy"].clone()), (json!(70), json!(80), json!(false), Value::Null));
    assert_eq!(wk["totals"], json!({ "cost": 1000, "priced": 1, "unpriced": 2 }));
    assert_eq!(wk["rows"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap()).collect::<Vec<_>>(), vec!["nori", "rice", "salmon"], "by name");
}

/// Twin: draws from before the window cover every day of it; counted stock
/// above the need buys nothing (0, not null); nothing priced is a null total.
#[test]
fn draws_before_the_window_cover_all_of_it() {
    let w = weeks();
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(-10 * DAY);
    log.receive_with("rice", 9000, &Meta::default()).unwrap();
    sell(&mut log, "rice", 50, "old", -5 * DAY);
    sell(&mut log, "rice", 2800, "o1", 10 * DAY);
    let j = log.journal().unwrap();
    let supplies = HashMap::from([("rice".to_string(), sup("rice", None))]);
    let buys = HashMap::from([("rice".to_string(), buy(0, None))]);
    let wk = block(&sales::Sales::default(), &j, &HashMap::new(), &supplies, &buys, &w);
    assert_eq!((wk["dataFrom"].clone(), wk["dataDays"].clone()), (json!("2026-10-26"), json!(28)));
    let r = &wk["rows"][0];
    assert_eq!((r["used"].clone(), r["weekly"].clone(), r["need"].clone(), r["toBuy"].clone()), (json!(2800), json!(700), json!(700), json!(0)));
    assert_eq!(wk["totals"], json!({ "cost": Value::Null, "priced": 0, "unpriced": 0 }), "nothing to buy is not an unpriced row");
}

/// No sale has drawn yet: there is no average, and nothing is listed as a
/// zero week. A write-off alone does not start the data.
#[test]
fn no_draw_yet_is_no_week() {
    let w = weeks();
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(DAY);
    log.receive_with("rice", 900, &Meta::default()).unwrap();
    log.append(&StockEvent::Wasted { item: "rice".into(), qty: 100, reason: WasteReason::Spoiled, by: "p".into() }).unwrap();
    let j = log.journal().unwrap();
    let wk = block(&sales::Sales::default(), &j, &HashMap::new(), &HashMap::from([("rice".to_string(), sup("rice", Some(1)))]), &HashMap::from([("rice".to_string(), buy(1, None))]), &w);
    assert_eq!((wk["dataFrom"].clone(), wk["dataDays"].clone(), wk["rows"].clone()), (Value::Null, json!(0), json!([])));
}

/// The arithmetic at its edges: rounded up, never below zero, a stored lead
/// past the card's bound clamped, sums too large for i64 saturated.
#[test]
fn the_need_rounds_up_and_is_bounded() {
    assert_eq!(weekly(1, 14), Some(1));
    assert_eq!(weekly(14, 14), Some(7));
    assert_eq!(weekly(0, 14), None);
    assert_eq!(weekly(5, 0), None);
    assert_eq!(need(14, 14, 0), Some(7));
    assert_eq!(need(14, 14, 7), Some(14));
    assert_eq!(need(14, 14, 99), need(14, 14, crate::services::operations::stock::suppliers::LEAD_MAX), "a lead past the bound is the bound");
    assert_eq!(need(14, 14, -3), need(14, 14, 0));
    assert_eq!(need(i64::MAX, 1, 30), Some(i64::MAX), "saturates, never wraps");
}

/// What share of sales draws nothing: revenue of dishes with no recipe (and
/// of products the catalogue no longer has) over all revenue, in per mille,
/// with the top dishes by revenue. Staff keep the share and the names; the
/// amounts go with every other revenue figure (`numbers_for_kitchen`).
#[test]
fn the_sales_no_recipe_draws_for_are_said() {
    let w = weeks();
    let dishes = HashMap::from([
        ("sake".to_string(), Dish { id: "sake".into(), name: "Sake".into(), lines: vec![DishLine { supply: "rice".into(), qty: 100.into(), gross_g: None, net_g: None, out_g: None }], leaves: vec![] }),
        ("tea".to_string(), Dish { id: "tea".into(), name: "Tea".into(), lines: vec![], leaves: vec![] }),
    ]);
    let orders = vec![
        json!({ "id": "o1", "created_at_ms": DAY, "status": "DELIVERED", "items": [{ "product_id": "sake", "quantity": 3, "unit_price": 1000 }, { "product_id": "tea", "quantity": 2, "unit_price": 500 }] }),
        json!({ "id": "o2", "created_at_ms": 2 * DAY, "status": "DELIVERED", "items": [{ "product_id": "gone", "quantity": 1, "unit_price": 500 }] }),
    ];
    let s = sales::fold(&orders, &dishes, &w);
    let j = StockLog::create_sized(64 * 1024).unwrap().journal().unwrap();
    let wk = block(&s, &j, &dishes, &HashMap::new(), &HashMap::new(), &w);
    let c = &wk["coverage"];
    assert_eq!((c["revenue"].clone(), c["noRecipe"]["revenue"].clone(), c["noRecipe"]["pm"].clone(), c["noRecipe"]["portions"].clone()), (json!(4500), json!(1500), json!(333), json!(3)));
    assert_eq!(c["top"], json!([{ "id": "tea", "name": "Tea", "revenue": 1000, "sold": 2 }, { "id": "gone", "name": "gone", "revenue": 500, "sold": 1 }]));
    let staff = crate::services::identity::staff::access::numbers_for_kitchen(json!({ "week": wk.clone() }));
    let sc = &staff["week"]["coverage"];
    assert_eq!((sc.get("revenue"), sc["noRecipe"].get("revenue"), sc["noRecipe"]["pm"].clone(), sc["top"][0]["name"].clone()), (None, None, json!(333), json!("Tea")));
    assert!(sc["top"][0].get("revenue").is_none());
    // Twin: nothing sold -- no share is claimed, not 0 %.
    let none = block(&sales::Sales::default(), &j, &dishes, &HashMap::new(), &HashMap::new(), &w);
    assert_eq!(none["coverage"]["noRecipe"]["pm"], Value::Null);
}

/// What each supply is bought by, from the catalogue: its first pack, its
/// supplier's lead (its own field, else who delivered it last), no ПФ and no
/// retired supply; a stored lead past the bound is clamped.
#[test]
fn how_each_supply_is_bought() {
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_supply("rice", r#"{"id":"rice","name":"Rice","unit":"g","kind":"food_ingredient","supplier":"Sea","packs":[{"name":"bag","qty":1000}]}"#);
    cat.set_supply("nori", r#"{"id":"nori","name":"Nori","unit":"g","kind":"food_ingredient"}"#);
    cat.set_supply("salmon", r#"{"id":"salmon","name":"Salmon","unit":"g","kind":"food_ingredient","supplier":"Far"}"#);
    cat.set_supply("old", r#"{"id":"old","name":"Old","unit":"g","kind":"food_ingredient","active":false}"#);
    cat.set_supply("pf", &format!(r#"{{"id":"pf","name":"Rice cooked","unit":"g","kind":"{}"}}"#, dowiz_hub::prep::KIND));
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(DAY);
    log.receive_with("nori", 100, &Meta { supplier: Some("sea".into()), ..Meta::default() }).unwrap();
    let j = log.journal().unwrap();
    let cards = [
        Card { id: "sea".into(), name: "Sea".into(), lead_days: 2, ..Card::default() },
        Card { id: "far".into(), name: "Far".into(), lead_days: 99, ..Card::default() },
    ];
    let b = buy_of(&cat, &cards, &j);
    let mut ids: Vec<&String> = b.keys().collect();
    ids.sort();
    assert_eq!(ids, vec!["nori", "rice", "salmon"], "no ПФ, no retired supply");
    assert_eq!(b["rice"], Buy { lead_days: 2, pack: Some(1000), supplier: "Sea".into() });
    assert_eq!(b["nori"], Buy { lead_days: 2, pack: None, supplier: "Sea".into() }, "its last delivery's supplier");
    assert_eq!(b["salmon"].lead_days, crate::services::operations::stock::suppliers::LEAD_MAX, "clamped");
    // Twin: no card at all -- the default lead, the supplier as typed.
    let none = buy_of(&cat, &[], &j);
    assert_eq!(none["salmon"], Buy { lead_days: crate::services::operations::stock::order_list::DEFAULT_LEAD_DAYS, pack: None, supplier: "Far".into() });
}

fn tirane() -> dowiz_hub::tz::Zone {
    dowiz_hub::tz::zone("Europe/Tirane").unwrap()
}

/// The four whole weeks, Monday to Sunday, before the venue's current week,
/// in the venue's days: Sunday 23:30 UTC is already Monday in Tirane.
#[test]
fn the_four_whole_weeks_are_the_venues() {
    let wed = day_number(20261125) * DAY + DAY / 2;
    let w = weeks_window(tirane(), wed).unwrap();
    assert_eq!((w.days[0], w.days[27], w.days.len()), (20261026, 20261122, 28));
    assert_eq!(w.starts[0], dowiz_hub::tz::start_of_local_day_ms(tirane(), day_number(20261026) * DAY + DAY / 2));
    let late_sunday_utc = day_number(20261122) * DAY + 23 * 3_600_000 + 1_800_000;
    assert_eq!(weeks_window(tirane(), late_sunday_utc).unwrap().days[27], 20261122, "Monday 00:30 local");
    // Twin: a Sunday is still inside its own week.
    assert_eq!(weeks_window(tirane(), day_number(20261122) * DAY + DAY / 2).unwrap().days[..1], [20261019]);
}

/// The whole fold over a catalogue, a stock log and orders: the last delivery
/// before the week, a pack from the card, the default lead, and a cube that
/// could not be read is said, not read as no sales.
#[test]
fn the_answer_folds_the_venue() {
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_supply("rice", r#"{"id":"rice","name":"Rice","unit":"g","kind":"food_ingredient","costPerBasis":50,"packs":[{"name":"bag","qty":1000}]}"#);
    cat.set_product("tea", r#"{"id":"tea","name":"Tea"}"#);
    let now = day_number(20261125) * DAY + DAY / 2;
    let monday = dowiz_hub::tz::start_of_local_day_ms(tirane(), day_number(20261116) * DAY + DAY / 2);
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(monday - DAY);
    log.receive_with("rice", 900, &Meta::default()).unwrap();
    sell(&mut log, "rice", 700, "o1", monday + 3_600_000);
    let orders = vec![json!({ "id": "o9", "created_at_ms": monday + 7_200_000, "status": "DELIVERED", "items": [{ "product_id": "tea", "quantity": 2, "unit_price": 500 }] })];
    let cold = |_: i64, _: i64| -> std::result::Result<std::collections::BTreeMap<i64, crate::services::analytics::cube::DayCube>, String> { Err("cube unread".into()) };
    let wk = answer(&orders, &cat, &log, now, &cold);
    assert_eq!((wk["from"].clone(), wk["dataFrom"].clone(), wk["dataDays"].clone()), (json!("2026-10-26"), json!("2026-11-16"), json!(7)));
    let r = &wk["rows"][0];
    // 700 a week; x (7 + 1) / 7 = 800; 200 free -> 600 -> one bag of 1000 at 50 per 100 g.
    assert_eq!((r["id"].clone(), r["weekly"].clone(), r["need"].clone(), r["leadDays"].clone(), r["pack"].clone()), (json!("rice"), json!(700), json!(800), json!(1), json!(1000)));
    assert_eq!((r["toBuy"].clone(), r["cost"].clone()), (json!(1000), json!(500)));
    assert_eq!((wk["coverage"]["noRecipe"]["pm"].clone(), wk["coverage"]["historyError"].clone()), (json!(1000), json!("cube unread")));
}
