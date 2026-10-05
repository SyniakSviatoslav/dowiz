//! P7 (W-PREP): what the kitchen's forecast will not use before a lot's date.
use super::*;
use dowiz_hub::stock::lots::Lot;

fn lot(item: &str, code: &str, expiry: Option<i64>, left: i64, seq: usize) -> Lot {
    Lot { item: item.into(), code: code.into(), expiry, supplier: None, doc: None, at: None, unit_cost: None, per: None, received: left.max(1), left, seq }
}

fn named(id: &str) -> Option<Supply> {
    Some(Supply { name: id.to_uppercase(), unit: "g".into(), low_at: 0 })
}

/// 150 g a day of salmon from today; nothing about anything else.
fn salmon_150(item: &str, expiry: i64) -> Option<i64> {
    (item == "salmon").then(|| (dowiz_hub::stock::meta::day_number(expiry) - dowiz_hub::stock::meta::day_number(TODAY) + 1).max(0) * 150)
}

const TODAY: i64 = 20261004;
const TOMORROW: i64 = 20261005;

/// THE ACCEPTANCE OF P7: 1 kg expiring tomorrow, forecast use 300 g -> 700 g.
#[test]
fn a_kilo_expiring_tomorrow_with_300_g_forecast_reports_700_g() {
    let a = lot("salmon", "A", Some(TOMORROW), 1000, 1);
    assert_eq!(salmon_150("salmon", TOMORROW), Some(300), "today and tomorrow");
    assert_eq!(surpluses(&[&a], TODAY, &salmon_150), vec![Some(700)]);
    let mut told: Vec<(&'static str, Value)> = vec![(EXPIRING, soon(&[&a], TODAY, &named).unwrap())];
    with_surplus(&mut told, &[&a], TODAY, &named, Some(&salmon_150 as &dyn Fn(&str, i64) -> Option<i64>), None);
    let row = &told[0].1["items"][0];
    assert_eq!((row["item"].as_str(), row["surplus"].as_i64(), row["qty"].as_i64()), (Some("salmon"), Some(700), Some(1000)));
    assert_eq!(told[0].1["forecast"], "ok");
    assert_eq!(surplus_line(row, "en"), " · ⚠ won't be used in time: 700 g → use first / special");
    assert!(surplus_line(row, "uk").contains("700 g") && surplus_line(row, "uk").contains("не встигнуть"));
}

/// The twin: a lot the forecast eats in full says nothing more.
#[test]
fn a_lot_the_forecast_uses_up_has_no_surplus_line() {
    let a = lot("salmon", "A", Some(TOMORROW), 200, 1);
    assert_eq!(surpluses(&[&a], TODAY, &salmon_150), vec![Some(0)]);
    let mut told: Vec<(&'static str, Value)> = vec![(EXPIRING, soon(&[&a], TODAY, &named).unwrap())];
    with_surplus(&mut told, &[&a], TODAY, &named, Some(&salmon_150 as &dyn Fn(&str, i64) -> Option<i64>), None);
    assert!(told[0].1["items"][0].get("surplus").is_none());
    assert_eq!(surplus_line(&told[0].1["items"][0], "en"), "");
}

/// First-expiry-first: the earlier lot absorbs the use before the later one.
#[test]
fn the_earlier_lot_takes_the_use_first() {
    let late = lot("salmon", "B", Some(20261006), 500, 2);
    let early = lot("salmon", "A", Some(TOMORROW), 400, 1);
    // Use until the 5th: 300 -> A keeps 100; until the 6th: 450, 300 already given -> B uses 150, keeps 350.
    assert_eq!(surpluses(&[&late, &early], TODAY, &salmon_150), vec![Some(350), Some(100)]);
}

#[test]
fn no_date_a_past_date_or_an_item_without_a_forecast_claims_nothing() {
    let undated = lot("salmon", "U", None, 900, 1);
    let past = lot("salmon", "P", Some(20261003), 900, 2);
    let tuna = lot("tuna", "T", Some(TOMORROW), 900, 3);
    assert_eq!(surpluses(&[&undated, &past, &tuna], TODAY, &salmon_150), vec![None, None, None]);
}

/// While the forecast is learning the event says so and invents nothing.
#[test]
fn a_learning_forecast_says_so_and_adds_no_surplus() {
    let a = lot("salmon", "A", Some(TOMORROW), 1000, 1);
    let mut told: Vec<(&'static str, Value)> = vec![("stock.low", json!({})), (EXPIRING, soon(&[&a], TODAY, &named).unwrap())];
    with_surplus(&mut told, &[&a], TODAY, &named, None, None);
    assert_eq!(told[1].1["forecast"], "learning");
    assert!(told[1].1["items"][0].get("surplus").is_none());
    assert!(told[0].1.get("forecast").is_none(), "only stock.expiring is touched");
    with_surplus(&mut told, &[&a], TODAY, &named, None, Some("the cube is unreadable".into()));
    assert_eq!(told[1].1["forecast"], "the cube is unreadable");
}

#[test]
fn every_language_has_its_surplus_words() {
    for l in dowiz_hub::lang::LANGS {
        let (h, a) = surplus_words(l);
        assert!(!h.is_empty() && !a.is_empty(), "{l}");
    }
    assert_eq!(surplus_words("xx"), surplus_words("en"));
}
