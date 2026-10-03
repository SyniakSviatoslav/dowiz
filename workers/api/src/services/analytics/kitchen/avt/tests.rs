//! Unexplained loss between two counts, over a real stock log: the live
//! probe's own numbers (count 1000, receive 500, sell 2 x 40, waste 20, count
//! 1300 -> 100 g unexplained), the sum that closes on the count's drift, and
//! the supplies that are never a loss.

use super::*;
use dowiz_hub::stock::journal::Entry;
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{PrepStage, StockEvent, StockLog, WasteReason};

fn priced() -> Meta {
    Meta { unit_cost: Some(2000), per: Some(1000), ..Meta::default() }
}

fn count(log: &mut StockLog, item: &str, observed: i64, at: i64) {
    let ev = StockEvent::Stocktake { item: item.into(), observed, stocktake_id: format!("st_{at}"), by: "p1".into() };
    log.set_clock(at);
    let expected = log.ledger().unwrap().level(item).on_hand;
    log.append_all_with(&[(ev, Meta { at: Some(at), session: Some(format!("st_{at}")), expected: Some(expected), ..Meta::default() })]).unwrap();
}

fn sell(log: &mut StockLog, item: &str, qty: i64, order: &str, at: i64) {
    log.set_clock(at);
    log.append_all(&[StockEvent::Reserved { item: item.into(), qty, order_id: order.into() }]).unwrap();
    log.append_all(&[StockEvent::Consumed { item: item.into(), qty, order_id: order.into() }]).unwrap();
}

/// The probe's story, on salmon, priced at 2 a gram.
fn probe_log() -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(10);
    log.receive_with("salmon", 1000, &priced()).unwrap();
    count(&mut log, "salmon", 1000, 20);
    log.set_clock(30);
    log.receive_with("salmon", 500, &priced()).unwrap();
    sell(&mut log, "salmon", 40, "o1", 40);
    sell(&mut log, "salmon", 40, "o2", 41);
    log.set_clock(50);
    log.append(&StockEvent::Wasted { item: "salmon".into(), qty: 20, reason: WasteReason::Spoiled, by: "p1".into() }).unwrap();
    count(&mut log, "salmon", 1300, 60);
    log
}

fn closing(entries: &[Entry], item: &str) -> i64 {
    let e = entries.iter().rev().find(|e| matches!(&e.ev, StockEvent::Stocktake { item: i, .. } if i == item)).unwrap();
    let StockEvent::Stocktake { observed, .. } = e.ev else { unreachable!() };
    e.expected() - observed
}

#[test]
fn the_probe_numbers_and_the_sum_closes_on_the_drift() {
    let j = probe_log().journal().unwrap();
    let ws = windows(&j.entries);
    let w = &ws["salmon"];
    assert_eq!(w.len(), 1, "two counts make one window");
    let w = &w[0];
    assert_eq!((w.opening, w.received, w.sold, w.wasted, w.prep, w.closing), (1000, 500, 80, 20, 0, 1300));
    assert_eq!((w.actual(), w.unexplained()), (200, 100));
    assert_eq!(w.unexplained(), closing(&j.entries, "salmon"), "unexplained IS minus the closing count's drift");
    assert_eq!((w.opened_at, w.closed_at), (Some(20), Some(60)));
    assert_eq!(value_of(w, "salmon", &|_, _| None), Some(200), "100 g at the average of 2 a gram");
}

fn ctx_report(j: &[Entry], linked: &[&str], revenue: i64) -> Value {
    let linked = |id: &str| linked.contains(&id);
    let name = |id: &str| Some((id.to_uppercase(), "g".to_string()));
    let list = |_: &str, q: i64| Some(q * 3);
    let all = |_: i64| true;
    let rec = |e: &Entry| json!(e.seq);
    report(j, &Ctx { linked: &linked, name: &name, list_value: &list, revenue: Some(revenue), in_window: &all, record: &rec })
}

#[test]
fn a_supply_no_recipe_names_is_never_a_loss_and_one_count_is_not_a_window() {
    let mut log = probe_log();
    // rice: counted twice with a "loss" -- but no recipe names it.
    count(&mut log, "rice", 5000, 70);
    count(&mut log, "rice", 4000, 80);
    // nori: a recipe names it, counted once.
    count(&mut log, "nori", 50, 90);
    let j = log.journal().unwrap();
    let r = ctx_report(&j.entries, &["salmon", "nori"], 10_000);
    let rows = r["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{r}");
    assert_eq!((rows[0]["id"].clone(), rows[0]["unexplained"].clone(), rows[0]["value"].clone()), (json!("salmon"), json!(100), json!(200)));
    assert_eq!(rows[0]["revenuePm"], json!(20), "200 of 10 000 = 20 per mille: under the flag");
    assert_eq!(rows[0]["flags"], json!([]));
    assert_eq!(r["notTracked"], json!([{ "id": "rice", "name": "RICE" }]), "listed apart, never in rows");
    assert_eq!(r["countTwice"], json!([{ "id": "nori", "name": "NORI" }]));
    assert_eq!(r["top"], json!(["salmon"]));
    assert_eq!(rows[0]["records"].as_array().unwrap().last(), Some(&json!(j.entries.iter().rposition(|e| e.meta.at == Some(60)).unwrap())), "the closing count is the last record");
    // The positive twin of "not tracked": the same rice, linked, IS a row.
    let r = ctx_report(&j.entries, &["salmon", "rice", "nori"], 10_000);
    assert!(r["rows"].as_array().unwrap().iter().any(|x| x["id"] == "rice" && x["unexplained"] == 1000));
}

#[test]
fn a_loss_above_thirty_per_mille_of_sales_is_flagged() {
    let j = probe_log().journal().unwrap();
    let r = ctx_report(&j.entries, &["salmon"], 5_000);
    assert_eq!((r["rows"][0]["revenuePm"].clone(), r["rows"][0]["flags"].clone()), (json!(40), json!(["over30pm"])));
    let r = ctx_report(&j.entries, &["salmon"], 6_667);
    assert_eq!(r["rows"][0]["flags"], json!([]), "29 per mille: not flagged");
}

#[test]
fn two_sigma_from_the_supplys_own_past_windows() {
    assert!(unusual(40, &[10, 12, 11]));
    assert!(!unusual(12, &[10, 12, 11]));
    assert!(!unusual(400, &[10, 12]), "fewer than three past windows: nothing is said");
    assert!(unusual(5, &[0, 0, 0]) && !unusual(0, &[0, 0, 0]), "a flat past: any change is unusual, none is not");
}

#[test]
fn prep_moves_food_between_supplies_and_both_sums_still_close() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(1);
    log.receive_with("whole", 2000, &priced()).unwrap();
    count(&mut log, "whole", 2000, 2);
    count(&mut log, "fillet", 0, 3);
    log.set_clock(4);
    log.append(&StockEvent::Produced { item: "whole".into(), qty: 1000, out: 550, stage: PrepStage::Clean, into: Some("fillet".into()), by: "p1".into() }).unwrap();
    sell(&mut log, "fillet", 100, "o9", 5);
    count(&mut log, "whole", 1000, 6);
    count(&mut log, "fillet", 430, 7);
    let j = log.journal().unwrap();
    let ws = windows(&j.entries);
    let (wh, fi) = (&ws["whole"][0], &ws["fillet"][0]);
    assert_eq!((wh.prep, wh.unexplained()), (1000, 0), "the board took it: explained");
    assert_eq!((fi.received, fi.sold, fi.unexplained()), (550, 100, 20), "550 came off, 100 sold, 430 counted: 20 gone");
    assert_eq!(wh.unexplained(), closing(&j.entries, "whole"));
    assert_eq!(fi.unexplained(), closing(&j.entries, "fillet"));
}

#[test]
fn only_losses_lead_and_at_most_five() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let ids: Vec<String> = (0..7).map(|i| format!("s{i}")).collect();
    for (i, id) in ids.iter().enumerate() {
        count(&mut log, id, 1000, 10 + i as i64);
    }
    for (i, id) in ids.iter().enumerate() {
        // s0 found 10 MORE than expected (a gain); s1..s6 lost 10 x i.
        count(&mut log, id, if i == 0 { 1010 } else { 1000 - 10 * i as i64 }, 100 + i as i64);
    }
    let j = log.journal().unwrap();
    let all: Vec<&str> = ids.iter().map(String::as_str).collect();
    let r = ctx_report(&j.entries, &all, 1_000_000);
    assert_eq!(r["top"], json!(["s6", "s5", "s4", "s3", "s2"]), "biggest money first, five, gains never");
    assert_eq!(r["rows"].as_array().unwrap().len(), 7, "the gain is still a row");
}
