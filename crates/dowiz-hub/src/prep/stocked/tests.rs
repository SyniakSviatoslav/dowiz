//! R2: the shelf first, the card for the rest -- on the operator's example.

use super::*;
use crate::prep::tests::kitchen;
use crate::prep::expand;

fn look(k: &[(String, String)]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| k.iter().find(|(i, _)| i == id).map(|(_, j)| j.clone())
}

fn run(roots: &[(&str, i128)], ready: &dyn Fn(&str) -> i64) -> Result<Vec<(String, i64)>, Refusal> {
    let k = kitchen();
    let s = look(&k);
    let card = |id: &str| card_from(&s, id);
    let quiet = |id: &str| s(id).is_some_and(|j| untracked(&j));
    let roots: Vec<(String, Rat)> = roots.iter().map(|(i, q)| (i.to_string(), Rat::new(*q, 1))).collect();
    plan(&roots, &World { card: &card, untracked: &quiet, ready }).map(|v| v.into_iter().map(|l| (l.item, l.uq)).collect())
}

#[test]
fn with_nothing_kept_ready_it_is_the_expansion_leaf_for_leaf() {
    let k = kitchen();
    let want: Vec<(String, i64)> = expand(&[("rice-seasoned".into(), 130), ("salt".into(), 1)], &look(&k)).unwrap().into_iter().map(|l| (l.item, l.uq)).collect();
    assert_eq!(run(&[("rice-seasoned", 130), ("salt", 1)], &|_| 0).unwrap(), want);
}

#[test]
fn a_ready_batch_is_taken_whole_when_it_covers_the_portion() {
    let got = run(&[("rice-seasoned", 130)], &|id| if id == "rice-seasoned" { 1000 * MICRO } else { 0 }).unwrap();
    assert_eq!(got, vec![("rice-seasoned".into(), 130 * MICRO)]);
}

#[test]
fn a_short_batch_gives_what_it_has_and_the_card_the_rest() {
    let got = run(&[("rice-seasoned", 130)], &|id| if id == "rice-seasoned" { 100 * MICRO } else { 0 }).unwrap();
    // 30 g from the card: 30/2100 of a batch -> rice 14.285714 g, mitsukan 3.571429 g
    // -> vinegar 2.857143, salt 0.178571, sugar 0.535714.
    assert_eq!(got, vec![("rice-dry".into(), 14_285_714), ("rice-seasoned".into(), 100 * MICRO), ("salt".into(), 178_571), ("sugar".into(), 535_714), ("vinegar".into(), 2_857_143)]);
}

#[test]
fn a_ready_product_inside_the_card_is_taken_too() {
    let got = run(&[("rice-seasoned", 130)], &|id| if id == "mitsukan" { 500 * MICRO } else { 0 }).unwrap();
    assert_eq!(got, vec![("mitsukan".into(), 15_476_190), ("rice-dry".into(), 61_904_762)], "no vinegar, salt or sugar: the sauce was ready");
}

#[test]
fn two_rolls_on_one_rice_take_the_batch_once_for_both() {
    let got = run(&[("rice-seasoned", 130), ("rice-seasoned", 130)], &|id| if id == "rice-seasoned" { 200 * MICRO } else { 0 }).unwrap();
    assert_eq!(got.iter().find(|(i, _)| i == "rice-seasoned").unwrap().1, 200 * MICRO);
    assert_eq!(got.iter().find(|(i, _)| i == "rice-dry").unwrap().1, 28_571_429, "60 g from the card: 60/2100 x 1000");
}

#[test]
fn a_cycle_is_refused_by_its_path_and_an_unknown_by_its_name() {
    let mut k = kitchen();
    k.push(("a".into(), r#"{"id":"a","kind":"prep","card":{"lines":[{"item":"b","qty":1}],"yield":1}}"#.into()));
    k.push(("b".into(), r#"{"id":"b","kind":"prep","card":{"lines":[{"item":"a","qty":1}],"yield":1}}"#.into()));
    let s = look(&k);
    let card = |id: &str| card_from(&s, id);
    let w = World { card: &card, untracked: &|_| false, ready: &|_| 0 };
    let e = plan(&[("a".into(), Rat::new(1, 1))], &w).unwrap_err();
    assert_eq!(e.to_string(), "semi-finished products name each other: root → a → b → a");
    assert_eq!(plan(&[("ghost".into(), Rat::new(1, 1))], &w).unwrap_err(), Refusal::Unknown("ghost".into()));
    // The twin: a known raw item is a leaf.
    assert_eq!(plan(&[("salt".into(), Rat::new(1, 1))], &w).unwrap().len(), 1);
}

#[test]
fn the_tree_a_sale_carries_names_every_card_and_the_water() {
    let k = kitchen();
    let t = tree_json(&look(&k), &[("rice-seasoned".into(), 130)]).unwrap();
    let mut ids: Vec<&String> = t["cards"].as_object().unwrap().keys().collect();
    ids.sort();
    assert_eq!(ids, vec!["mitsukan", "rice-seasoned"]);
    assert_eq!(t["untracked"], serde_json::json!(["water"]));
    assert_eq!(tree_json(&look(&k), &[("salt".into(), 2)]), None, "a dish of raw items carries no tree");
}
