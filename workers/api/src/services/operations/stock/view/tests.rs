//! The Stock screen's extras, folded from a real log: lots with their days
//! left (I6), the price history per basis (I2/I3), the measured yields (I5),
//! the last count and the sessions (I4).

use super::*;
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{PrepStage, StockLog, WasteReason};

fn log() -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(1_790_000_000_000);
    let lot = |code: &str, exp: i64, cost: i64| Meta {
        lot: Some(code.into()), expiry: Some(exp), unit_cost: Some(cost), per: Some(1000), supplier: Some("Sea".into()), ..Meta::default()
    };
    log.receive_with("salmon", 1000, &lot("A", 20260925, 3000)).unwrap();
    log.receive_with("salmon", 1000, &lot("B", 20260927, 3400)).unwrap();
    log.receive_with("salmon", 1000, &lot("C", 20261010, 3200)).unwrap();
    log.append(&StockEvent::Produced { item: "salmon".into(), qty: 1000, out: 560, stage: PrepStage::Clean, into: None, by: "p".into() })
        .unwrap();
    log.append_with(
        &StockEvent::Stocktake { item: "salmon".into(), observed: 2900, stocktake_id: "s1".into(), by: "p".into() },
        &Meta { session: Some("s1".into()), expected: Some(3000), ..Meta::default() },
    )
    .unwrap();
    log
}

#[test]
fn a_supply_shows_its_lots_prices_yields_and_last_count() {
    let j = log().journal().unwrap();
    let x = extras("salmon", 100, &j, 20260926);
    // The count took 100 g off the first-expiring lot.
    let lots = x["lots"].as_array().unwrap();
    assert_eq!(lots.iter().map(|l| (l["code"].as_str().unwrap(), l["left"].as_i64().unwrap(), l["daysLeft"].as_i64().unwrap())).collect::<Vec<_>>(),
               vec![("A", 900, -1), ("B", 1000, 1), ("C", 1000, 14)]);
    assert_eq!(lots[0]["expiry"], json!("2026-09-25"));
    assert_eq!((x["expiring"].clone(), x["expired"].clone()), (json!(2), json!(1)));
    // Prices per 100 g: 300, 340, 320; the average is 320 per 100 g.
    let per: Vec<i64> = x["prices"].as_array().unwrap().iter().map(|p| p["perBasis"].as_i64().unwrap()).collect();
    assert_eq!(per, vec![300, 340, 320]);
    assert_eq!(x["wac"], json!(320));
    assert_eq!((x["measuredCleanPm"].clone(), x["measuredCookPm"].clone()), (json!(560), Value::Null));
    assert_eq!((x["lastCount"]["drift"].clone(), x["lastCount"]["value"].clone()), (json!(-100), json!(-320)));
    let kinds: Vec<&str> = x["moves"].as_array().unwrap().iter().map(|m| m["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, vec!["received", "received", "received", "produced", "stocktake"], "oldest first, the last eight");
    // An unknown supply is empty, never an error.
    let none = extras("nori", 100, &j, 20260926);
    assert_eq!((none["lots"].clone(), none["wac"].clone()), (json!([]), Value::Null));
}

#[test]
fn the_last_movements_and_the_sessions() {
    let mut l = log();
    l.append(&StockEvent::Reserved { item: "salmon".into(), qty: 10, order_id: "o1".into() }).unwrap();
    l.append(&StockEvent::Wasted { item: "salmon".into(), qty: 5, reason: WasteReason::Dropped, by: "p_cook".into() }).unwrap();
    let j = l.journal().unwrap();
    let (recent, suppliers) = recent_and_suppliers(&j);
    assert_eq!(suppliers, vec!["Sea".to_string()]);
    assert_eq!(recent[0]["kind"], json!("wasted"), "newest first");
    assert_eq!((recent[0]["reason"].clone(), recent[0]["by"].clone()), (json!("dropped"), json!("p_cook")));
    assert!(recent.iter().all(|r| r["kind"] != "reserved"), "the lifecycle's holds are not movements");
    assert_eq!(recent[2]["yieldPm"], json!(560));
    let s = sessions(&j, 5);
    assert_eq!(s, vec![json!({ "session": "s1", "at": 1_790_000_000_000i64, "lines": 1, "value": -320 })]);
}

/// W-NOM: a supply deleted and re-created under its id shows only its new
/// life -- no old lots, price, count or movements -- and the deletion is a
/// row of the recent history, named "removed".
#[test]
fn a_recreated_supply_shows_only_what_came_after_its_deletion() {
    let mut l = log();
    l.append(&StockEvent::Removed { item: "salmon".into(), by: "p_owner".into() }).unwrap();
    l.append(&StockEvent::Received { item: "salmon".into(), qty: 400 }).unwrap();
    let j = l.journal().unwrap();
    let x = extras("salmon", 100, &j, 20260926);
    let kinds: Vec<&str> = x["moves"].as_array().unwrap().iter().map(|m| m["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, vec!["received"]);
    assert_eq!((x["prices"].clone(), x["wac"].clone(), x["lastCount"].clone()), (json!([]), Value::Null, Value::Null));
    assert_eq!(x["lots"].as_array().unwrap().len(), 1, "one unlabelled lot of the new delivery");
    let (recent, _) = recent_and_suppliers(&j);
    assert_eq!((recent[1]["kind"].clone(), recent[1]["qty"].clone()), (json!("removed"), json!(0)));
    // Its twin: without a deletion the whole history shows.
    assert_eq!(extras("salmon", 100, &log().journal().unwrap(), 20260926)["moves"].as_array().unwrap().len(), 5);
}
