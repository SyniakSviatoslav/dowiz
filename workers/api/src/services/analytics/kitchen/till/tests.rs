//! Till lines attributed to a dish at READ time: the owner's mapping, else
//! the one dish of the line's name; the rest stay their own rows, named.

use super::super::sales::{fold_with, DishLine, Window};
use crate::services::analytics::cube::DayCube;
use super::*;
use serde_json::json;

fn w() -> Window {
    Window { starts: vec![0, 100], end: 200, days: vec![20261101, 20261102] }
}
fn dishes() -> HashMap<String, Dish> {
    let line = DishLine { supply: "salmon".into(), qty: 100.into(), gross_g: Some(100), net_g: Some(100), out_g: Some(100) };
    let d = |id: &str, name: &str| (id.to_string(), Dish { id: id.into(), name: name.into(), lines: vec![line.clone()], leaves: vec![] });
    HashMap::from([d("sake", "Sake roll"), d("maki", "Maki"), d("maki-2", "Maki")])
}
fn catalogue() -> Vec<(String, String)> {
    let p = |id: &str, name: &str| (id.to_string(), json!({ "id": id, "name": name }).to_string());
    vec![p("sake", "Sake roll"), p("maki", "Maki"), p("maki-2", "Maki")]
}
fn till(at: i64, code: &str, name: &str, q: i64, price: i64) -> Value {
    json!({ "id": format!("ebills:{code}-{at}"), "created_at_ms": at, "status": "DELIVERED",
            "items": [{ "product_id": format!("ebills:{code}"), "name": name, "quantity": q, "unit_price": price }] })
}
fn web(at: i64, q: i64) -> Value {
    json!({ "id": format!("w{at}"), "created_at_ms": at, "status": "DELIVERED",
            "items": [{ "product_id": "sake", "quantity": q, "unit_price": 900, "unit_cost": 300 }] })
}
fn no_map(_: &str) -> Option<String> {
    None
}
fn map_56(c: &str) -> Option<String> {
    (c == "56").then(|| "sake".to_string())
}

/// The owner mapped code 56 to the Sake roll after the sales were imported:
/// the history counts them under the dish, unstamped (today's recipe), and
/// the ingredient side -- what the stored orders drew -- does not move.
#[test]
fn a_code_mapped_later_is_counted_under_its_dish_and_draws_nothing_new() {
    let orders = vec![web(10, 2), till(20, "56", "SAKE MAKI 8", 3, 800), till(150, "56", "SAKE MAKI 8", 1, 800)];
    let mut s = fold_with(&orders, &BTreeMap::new(), &dishes(), &w());
    let uses = s.uses.clone();
    let names = TillNames { map: &map_56, i18n: &[] };
    let t = attribute(&mut s, &dishes(), &orders, &catalogue(), &names);
    assert!(!s.dishes.contains_key("ebills:56"), "the till row is folded into the dish");
    let r = &s.dishes["sake"];
    assert_eq!((r.sold, r.revenue, r.by_day.clone()), (6, 1800 + 3200, vec![5, 1]));
    assert_eq!((r.stamped, r.stamped_cogs), (2, 600), "only the web portions carry a stamp");
    assert_eq!(t.attributed.get("sake"), Some(&4));
    assert_eq!(s.uses, uses, "stock and draws are never re-drawn");
    assert!(t.unmatched.is_empty());
}

/// No mapping: the one dish whose name (case, accents folded) or whose
/// translation is the till's. Twin: a mapping wins over a name.
#[test]
fn a_name_matches_one_dish_and_a_mapping_wins_over_it() {
    let orders = vec![till(10, "7", "sake ROLL", 1, 800), till(11, "8", "Ролл саке", 2, 800)];
    let i18n = vec![("uk/product/sake/name".to_string(), "Ролл саке".to_string())];
    let mut s = fold_with(&orders, &BTreeMap::new(), &dishes(), &w());
    let t = attribute(&mut s, &dishes(), &orders, &catalogue(), &TillNames { map: &no_map, i18n: &i18n });
    assert_eq!((s.dishes["sake"].sold, t.attributed.get("sake")), (3, Some(&3)));
    let map_7 = |c: &str| (c == "7").then(|| "maki".to_string());
    let mut s = fold_with(&orders, &BTreeMap::new(), &dishes(), &w());
    attribute(&mut s, &dishes(), &orders, &catalogue(), &TillNames { map: &map_7, i18n: &i18n });
    assert_eq!((s.dishes["maki"].sold, s.dishes["sake"].sold), (1, 2), "code 7 is the owner's maki, whatever its name says");
}

/// Two dishes under one name, no dish of that name, a mapping to a dish
/// that is gone: each stays its own row, in revenue, named, by revenue.
#[test]
fn what_matches_no_one_dish_stays_its_own_row_and_is_named() {
    let gone = |c: &str| (c == "9").then(|| "deleted-dish".to_string());
    let orders = vec![till(10, "3", "Maki", 1, 500), till(11, "4", "Tempura", 4, 700), till(12, "9", "Old roll", 1, 100)];
    let mut s = fold_with(&orders, &BTreeMap::new(), &dishes(), &w());
    let revenue: i64 = s.days.iter().map(|d| d.revenue).sum();
    let t = attribute(&mut s, &dishes(), &orders, &catalogue(), &TillNames { map: &gone, i18n: &[] });
    assert!(t.attributed.is_empty(), "{:?}", t.attributed);
    for code in ["ebills:3", "ebills:4", "ebills:9"] {
        assert!(s.dishes.contains_key(code), "{code} keeps its row");
    }
    assert_eq!(s.days.iter().map(|d| d.revenue).sum::<i64>(), revenue, "still revenue");
    let rows: Vec<(String, String, i64, i64, String)> = t
        .unmatched
        .iter()
        .map(|u| (u["code"].as_str().unwrap().into(), u["name"].as_str().unwrap().into(), u["portions"].as_i64().unwrap(), u["revenue"].as_i64().unwrap(), u["why"].as_str().unwrap().into()))
        .collect();
    assert_eq!(
        rows,
        vec![
            ("4".into(), "Tempura".into(), 4, 2800, "no_dish".into()),
            ("3".into(), "Maki".into(), 1, 500, "two_dishes".into()),
            ("9".into(), "Old roll".into(), 1, 100, "no_dish".into()),
        ]
    );
}

/// A day the hot log no longer holds: the cube keeps the code, not the
/// name, so only the owner's mapping can attribute it. Twin: unmapped stays.
#[test]
fn an_archived_till_row_is_attributed_by_the_mapping_alone() {
    let mut day = DayCube::default();
    day.m.insert("ebills:56".into(), [2, 1600, 0, 0]);
    day.m.insert("ebills:77".into(), [1, 500, 0, 0]);
    let cold = BTreeMap::from([(20261101, day)]);
    let mut s = fold_with(&[], &cold, &dishes(), &w());
    let t = attribute(&mut s, &dishes(), &[], &catalogue(), &TillNames { map: &map_56, i18n: &[] });
    assert_eq!((s.dishes["sake"].sold, s.dishes["sake"].by_day.clone()), (2, vec![2, 0]));
    assert_eq!(t.unmatched.len(), 1);
    assert_eq!((t.unmatched[0]["code"].as_str(), t.unmatched[0]["name"].as_str()), (Some("77"), Some("")));
}
