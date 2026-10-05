//! R13's laws (W-LOST): an option's recipe joins the reservation, a dish
//! without one reserves what it always did, and the dish's own `bom` is never
//! confused with an option's.

use super::*;
use crate::stock::{draws_for, StockEvent, StockLog};

const ROLL: &str = r#"{"id":"roll","price":800,"bom":[{"supply":"salmon","qty":40},{"supply":"rice","qty":90}],
  "modifierGroups":[{"id":"extra","name":"Extras","min":0,"max":3,"options":[
    {"id":"xsalmon","name":"Extra salmon","priceDelta":200},
    {"id":"wasabi","name":"Wasabi","priceDelta":0},
    {"id":"noavo","name":"No avocado","priceDelta":-50}]}]}"#;

fn ids(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn with_extra() -> String {
    let mut p: Value = serde_json::from_str(ROLL).unwrap();
    set_option_bom(&mut p, "xsalmon", &[BomLine::whole("salmon", 20)], 100_000).unwrap();
    p.to_string()
}

/// What the shelf holds for one order of `lines`, per supply.
fn reserved(lines: &[(String, i64)]) -> (i64, i64) {
    let mut log = StockLog::create_sized(16 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "salmon".into(), qty: 1000 }).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 1000 }).unwrap();
    log.append_draws_split(&draws_for("o1", lines)).unwrap();
    let led = log.ledger().unwrap();
    (led.level("salmon").reserved, led.level("rice").reserved)
}

fn basket(dish: &str, chosen: &[&str], qty: i64) -> Vec<(String, i64)> {
    let mut lines = vec![(dish.to_string(), qty)];
    lines.extend(option_line("roll", dish, &ids(chosen), qty));
    lines
}

#[test]
fn extra_salmon_reserves_twenty_grams_more() {
    let dish = with_extra();
    let (plain, _) = reserved(&basket(&dish, &[], 1));
    let (extra, rice) = reserved(&basket(&dish, &["xsalmon"], 1));
    assert_eq!((plain, extra), (40, 60), "the option's 20 g joins the roll's 40 g");
    assert_eq!(rice, 90, "the rest of the recipe is unchanged");
    assert_eq!(reserved(&basket(&dish, &["xsalmon", "wasabi"], 2)).0, 120, "portions multiply the option too");
}

#[test]
fn a_dish_without_option_recipes_reserves_what_it_always_did() {
    assert_eq!(option_line("roll", ROLL, &ids(&["wasabi"]), 1), None, "no recipe, no price: no line");
    assert_eq!(option_line("roll", ROLL, &[], 1), None);
    assert_eq!(reserved(&basket(ROLL, &["wasabi"], 1)), (40, 90));
    // A priced option with no recipe adds a line that draws nothing: the
    // lost-sale price reads its delta.
    let (line, q) = option_line("roll", ROLL, &ids(&["noavo"]), 2).unwrap();
    assert_eq!(q, 2);
    let v: Value = serde_json::from_str(&line).unwrap();
    assert_eq!((v["of"].as_str(), v["delta"].as_i64(), v.get("id")), (Some("roll"), Some(-50), None), "no id: the cost stamp skips it");
    assert_eq!(reserved(&basket(ROLL, &["noavo"], 2)), (80, 180));
}

#[test]
fn the_dish_recipe_is_never_read_from_an_option() {
    let dish = with_extra();
    assert_eq!(bom_of(&dish).len(), 2, "the dish's two lines");
    let only_option = r#"{"id":"cola","optionBom":{"ice":[{"supply":"ice","qty":50}]},"modifierGroups":[{"id":"g","options":[{"id":"ice"}]}]}"#;
    assert!(bom_of(only_option).is_empty(), "an option's recipe is not the dish's");
    let first = r#"{"optionBom":{"x":[{"supply":"salmon","qty":20}]},"bom":[{"supply":"rice","qty":90}]}"#;
    assert_eq!(bom_of(first).iter().map(|l| l.supply.as_str()).collect::<Vec<_>>(), vec!["rice"], "written first, still not the dish's");
}

#[test]
fn setting_an_option_recipe_is_checked_and_clearing_it_restores_the_record() {
    let mut p: Value = serde_json::from_str(ROLL).unwrap();
    let before = p.to_string();
    assert!(set_option_bom(&mut p, "nope", &[BomLine::whole("salmon", 20)], 100_000).is_err(), "not an option of this dish");
    assert!(set_option_bom(&mut p, "xsalmon", &[BomLine::whole("salmon", 0)], 100_000).is_err(), "zero is not a quantity");
    assert!(set_option_bom(&mut p, "xsalmon", &[BomLine::whole("salmon", 200_000)], 100_000).is_err(), "past the bound");
    assert!(set_option_bom(&mut p, "xsalmon", &[BomLine::whole("a", 1), BomLine::whole("a", 2)], 100_000).is_err(), "twice");
    let many: Vec<BomLine> = (0..=LINES_MAX).map(|i| BomLine::whole(format!("s{i}"), 1)).collect();
    assert!(set_option_bom(&mut p, "xsalmon", &many, 100_000).is_err(), "too many lines");
    assert_eq!(p.to_string(), before, "every refusal changed nothing");
    set_option_bom(&mut p, "xsalmon", &[BomLine::whole("salmon", 20)], 100_000).unwrap();
    assert_eq!(option_bom(&p, "xsalmon"), vec![BomLine::whole("salmon", 20)]);
    set_option_bom(&mut p, "xsalmon", &[], 100_000).unwrap();
    assert_eq!(p.to_string(), before, "the last line cleared: the key is gone, the record is what it was");
}

#[test]
fn every_option_is_listed_with_its_group() {
    let o = options_of(ROLL);
    assert_eq!(o.len(), 3);
    assert_eq!(o[0], ("extra".into(), "Extras".into(), "xsalmon".into(), "Extra salmon".into()));
}
