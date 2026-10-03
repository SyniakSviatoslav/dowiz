//! Par, suggestion and the grouped list, over a real stock log.

use super::*;
use crate::services::operations::stock::suppliers::{Card, Open};
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{StockEvent, StockLog, WasteReason};

const DAY: i64 = 86_400_000;
const NOW: i64 = 1_790_000_000_000;

#[test]
fn par_and_suggestion_round_up_to_packs() {
    assert_eq!(adu(300, 2), Some(150));
    assert_eq!(adu(301, 2), Some(151), "rounded up");
    assert_eq!(adu(0, 14), None);
    assert_eq!(cycle_days(&[]), 7);
    assert_eq!(cycle_days(&[1, 4]), 4, "two deliveries a week: 3.5 days, up");
    assert_eq!(cycle_days(&[1, 2, 3, 4, 5, 6, 7]), 1);
    assert_eq!(par(150, 1, 4), 750);
    assert_eq!(suggest(750, 200, 0, None), 550);
    assert_eq!(suggest(750, 200, 0, Some(500)), 1000, "two packs of 500, never 1.1");
    assert_eq!(suggest(750, 200, 300, Some(500)), 500, "what is on its way counts");
    assert_eq!(suggest(750, 800, 0, Some(500)), 0, "above par: nothing");
    assert_eq!(suggest(750, -300, 0, None), 750, "a minus is an empty shelf");
}

fn sell(log: &mut StockLog, item: &str, qty: i64, order: &str, at: i64) {
    log.set_clock(at);
    log.append_all(&[StockEvent::Reserved { item: item.into(), qty, order_id: order.into() }]).unwrap();
    log.append_all(&[StockEvent::Consumed { item: item.into(), qty, order_id: order.into() }]).unwrap();
}

#[test]
fn use_is_what_left_the_shelf_in_the_last_two_weeks() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(NOW - 30 * DAY);
    log.receive_with("salmon", 10_000, &Meta::default()).unwrap();
    sell(&mut log, "salmon", 999, "old", NOW - 20 * DAY); // before the window
    sell(&mut log, "salmon", 700, "a", NOW - 3 * DAY);
    log.set_clock(NOW - DAY);
    log.append(&StockEvent::Wasted { item: "salmon".into(), qty: 140, reason: WasteReason::Spoiled, by: "p".into() }).unwrap();
    let j = log.journal().unwrap();
    let u = used_since(&j, NOW - ADU_DAYS * DAY);
    assert_eq!(u["salmon"], (840, NOW - 30 * DAY), "the sale and the write-off of the window; not the old sale");
}

fn item(id: &str, supplier: &str, available: i64, pack: Option<i64>) -> Item {
    Item { id: id.into(), name: id.to_uppercase(), unit: "g".into(), supplier: supplier.into(), available, counted: true, pack: pack.map(|q| ("box".into(), q)) }
}

#[test]
fn the_list_groups_by_supplier_with_its_own_lead_and_days() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(NOW - 20 * DAY);
    for s in ["salmon", "rice", "nori"] {
        log.receive_with(s, 100_000, &Meta::default()).unwrap();
    }
    // 1400 g of salmon and 2800 g of rice over the 14 days: 100 and 200 a day.
    sell(&mut log, "salmon", 1400, "o1", NOW - 2 * DAY);
    sell(&mut log, "rice", 2800, "o2", NOW - 2 * DAY);
    let j = log.journal().unwrap();
    let sea = Card { id: "sea".into(), name: "Sea".into(), days: vec![1, 4], lead_days: 1, ..Card::default() };
    let items = vec![item("salmon", "Sea", 300, Some(1000)), item("rice", "", 500, None), item("nori", "sea", 50, None)];
    let mut open = OnOrder::new();
    open.insert("nori".into(), Open { qty: 20, since: Some(NOW - DAY) });
    let l = list(&items, &j, &[sea], &open, NOW);
    let g = l["groups"].as_array().unwrap();
    assert_eq!(g.len(), 2, "the card's group, then the supplies with none");
    assert_eq!(g[0]["supplier"]["id"], "sea");
    let salmon = &g[0]["lines"][0];
    // par = 100 x (1 lead + 4 days between Monday and Thursday) = 500; 300 free -> 200 -> one box of 1000.
    assert_eq!((salmon["adu"].clone(), salmon["par"].clone(), salmon["suggest"].clone()), (json!(100), json!(500), json!(1000)));
    assert_eq!(g[0]["lines"][1]["id"], "nori", "matched by id, nothing used but 20 on order: shown");
    assert_eq!(g[0]["lines"][1]["suggest"], json!(0));
    let rice = &g[1]["lines"][0];
    // no card: a day's lead + a week; 200 x 8 = 1600 - 500 = 1100.
    assert_eq!((g[1]["supplier"].clone(), rice["par"].clone(), rice["suggest"].clone()), (Value::Null, json!(1600), json!(1100)));
}

#[test]
fn an_uncounted_supply_gets_no_suggestion() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    sell(&mut log, "rice", 1400, "o", NOW - DAY);
    let j = log.journal().unwrap();
    let mut it = item("rice", "", -1400, None);
    it.counted = false;
    let l = list(&[it], &j, &[], &OnOrder::new(), NOW);
    assert_eq!(l["groups"][0]["lines"][0]["suggest"], Value::Null, "count it first");
    // A supply used for two days only is averaged over those two days, not fourteen.
    assert_eq!(l["groups"][0]["lines"][0]["adu"], json!(700));
}
