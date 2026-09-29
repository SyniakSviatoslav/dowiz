//! The operator's example (SPEC-SEMI-FINISHED §c), every refusal beside its
//! twin, and the readers that must not change for a catalogue without cards.

use super::*;

/// Mitsukan: vinegar 800 + salt 50 + sugar 150 -> 1000 g. Seasoned rice: dry
/// rice 1000 + water 1100 + mitsukan 250 -> 2100 g. Philadelphia: 130 g of it.
pub(crate) fn kitchen() -> Vec<(String, String)> {
    let raw = |id: &str, unit: &str| (id.to_string(), format!(r#"{{"id":"{id}","name":"{id}","unit":"{unit}","kind":"food_ingredient"}}"#));
    vec![
        raw("vinegar", "ml"),
        raw("salt", "g"),
        raw("sugar", "g"),
        raw("rice-dry", "g"),
        ("water".into(), r#"{"id":"water","name":"Water","unit":"ml","kind":"food_ingredient","untracked":true}"#.into()),
        ("mitsukan".into(), r#"{"id":"mitsukan","name":"Mitsukan","unit":"g","kind":"prep","card":{"lines":[{"item":"vinegar","qty":800},{"item":"salt","qty":50},{"item":"sugar","qty":150}],"yield":1000}}"#.into()),
        ("rice-seasoned".into(), r#"{"id":"rice-seasoned","name":"Rice seasoned","unit":"g","kind":"prep","card":{"lines":[{"item":"rice-dry","qty":1000},{"item":"water","qty":1100},{"item":"mitsukan","qty":250}],"yield":2100}}"#.into()),
    ]
}
pub(crate) const PHILADELPHIA: &str = r#"{"id":"philadelphia","name":"Philadelphia","price":650,"bom":[{"supply":"rice-seasoned","qty":130}]}"#;

fn lookup(list: &[(String, String)]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| list.iter().find(|(i, _)| i == id).map(|(_, j)| j.clone())
}

#[test]
fn one_philadelphia_expands_to_the_operators_grams() {
    let k = kitchen();
    let leaves = expand(&[("rice-seasoned".into(), 130)], &lookup(&k)).unwrap();
    let got: Vec<(&str, i64)> = leaves.iter().map(|l| (l.item.as_str(), l.uq)).collect();
    // 130/2100 of the batch: rice 61.904 762 g, mitsukan 15.476 190 g -> vinegar
    // 12.380 952, salt 0.773 810, sugar 2.321 429. Water is untracked: no leaf.
    assert_eq!(got, vec![("rice-dry", 61_904_762), ("salt", 773_810), ("sugar", 2_321_429), ("vinegar", 12_380_952)]);
    // A raw line beside it stays whole, and a dish with no card passes through.
    let mixed = expand(&[("rice-seasoned".into(), 130), ("salt".into(), 1)], &lookup(&k)).unwrap();
    assert_eq!(mixed.iter().find(|l| l.item == "salt").unwrap().uq, 1_773_810, "summed over both paths, rounded once");
}

#[test]
fn for_ledger_rewrites_the_bom_to_leaves_and_leaves_a_plain_dish_alone() {
    let k = kitchen();
    let (j, why) = for_ledger(&lookup(&k), PHILADELPHIA);
    assert_eq!(why, None);
    let bom = crate::stock::bom_of(&j);
    assert_eq!(bom.iter().map(|l| (l.supply.as_str(), l.uq, l.qty)).collect::<Vec<_>>(), vec![("rice-dry", 61_904_762, 62), ("salt", 773_810, 1), ("sugar", 2_321_429, 2), ("vinegar", 12_380_952, 12)]);
    assert!(j.contains(r#""id":"philadelphia""#), "the stamp still finds the dish by id");
    let plain = r#"{"id":"maki","bom":[{"supply":"salt","qty":2}]}"#;
    assert_eq!(for_ledger(&lookup(&k), plain), (plain.to_string(), None), "no card in it: the same bytes");
    // A dish whose card cannot expand is answered as it is, WITH the reason.
    let broken = r#"{"id":"x","bom":[{"supply":"ghost-prep","qty":1}]}"#;
    let mut k2 = k.clone();
    k2.push(("ghost-prep".into(), r#"{"id":"ghost-prep","kind":"prep","card":{"lines":[{"item":"nobody","qty":1}],"yield":1}}"#.into()));
    let (same, why) = for_ledger(&lookup(&k2), broken);
    assert_eq!(same, broken);
    assert_eq!(why, Some(Refusal::Unknown("nobody".into())));
}

#[test]
fn the_cost_follows_the_raw_price_through_the_tree_with_nothing_saved() {
    let k = kitchen();
    // Minor units (lek) per base unit, in millionths: rice 200/kg, vinegar
    // 150/l, salt 50/kg, sugar 120/kg.
    let prices = |rice: i128| move |id: &str| -> Option<i128> {
        Some(match id {
            "rice-dry" => rice,
            "vinegar" => 150_000,
            "salt" => 50_000,
            "sugar" => 120_000,
            _ => return None,
        })
    };
    let one = |id: &str| vec![(id.to_string(), 1)];
    let p = prices(200_000);
    assert_eq!(cost_micro(&one("mitsukan"), &lookup(&k), &p), Ok(Some(140_500)), "(800*0.15 + 50*0.05 + 150*0.12) / 1000 lek per g");
    assert_eq!(cost_micro(&one("rice-seasoned"), &lookup(&k), &p), Ok(Some(111_964)), "(1000*0.2 + 250*0.1405) / 2100, water at 0");
    let dish = vec![("rice-seasoned".to_string(), 130)];
    assert_eq!(cost_micro(&dish, &lookup(&k), &p), Ok(Some(14_555_357)), "130 g of it: 14.555 lek");
    // The dry rice goes up to 300/kg: the same call, a new number, no write.
    let p2 = prices(300_000);
    assert_eq!(cost_micro(&one("rice-seasoned"), &lookup(&k), &p2), Ok(Some(159_583)));
    assert_eq!(cost_micro(&dish, &lookup(&k), &p2), Ok(Some(20_745_833)));
    // A leaf without a price: no cost, not a partial one.
    let no_salt = |id: &str| if id == "salt" { None } else { p(id) };
    assert_eq!(cost_micro(&dish, &lookup(&k), &no_salt), Ok(None));
}

#[test]
fn a_cycle_is_refused_with_its_path_and_a_tree_without_one_passes() {
    let mut k = kitchen();
    k.push(("a".into(), r#"{"id":"a","kind":"prep","card":{"lines":[{"item":"b","qty":1}],"yield":1}}"#.into()));
    k.push(("b".into(), r#"{"id":"b","kind":"prep","card":{"lines":[{"item":"salt","qty":1}],"yield":1}}"#.into()));
    let good = Card { lines: vec![Line { item: "a".into(), qty: 1 }], yield_qty: 1 };
    assert_eq!(check_card("c", &good, &k), Ok(()), "c -> a -> b -> salt");
    // b -> a would close a -> b -> a.
    let back = Card { lines: vec![Line { item: "a".into(), qty: 1 }], yield_qty: 1 };
    match check_card("b", &back, &k) {
        Err(Refusal::Tree(tree::Stop::Cycle(path))) => assert_eq!(path, vec!["b", "a", "b"]),
        other => panic!("{other:?}"),
    }
    let me = Card { lines: vec![Line { item: "a".into(), qty: 1 }], yield_qty: 1 };
    assert_eq!(check_card("a", &me, &k), Err(Refusal::Itself), "a -> a");
    let unknown = Card { lines: vec![Line { item: "ghost".into(), qty: 1 }], yield_qty: 1 };
    assert_eq!(check_card("c", &unknown, &k), Err(Refusal::Unknown("ghost".into())));
    let twice = Card { lines: vec![Line { item: "salt".into(), qty: 1 }, Line { item: "salt".into(), qty: 2 }], yield_qty: 1 };
    assert_eq!(check_card("c", &twice, &k), Err(Refusal::Twice("salt".into())));
    assert_eq!(check_card("c", &Card { lines: vec![], yield_qty: 1 }, &k), Err(Refusal::Lines));
    let zero = Card { lines: vec![Line { item: "salt".into(), qty: 0 }], yield_qty: 1 };
    assert!(matches!(check_card("c", &zero, &k), Err(Refusal::Bound(_))));
    let big = Card { lines: vec![Line { item: "salt".into(), qty: 1 }], yield_qty: QTY_MAX + 1 };
    assert!(matches!(check_card("c", &big, &k), Err(Refusal::Bound(_))));
}

#[test]
fn the_depth_cap_counts_the_chain_above_and_below() {
    // p1 -> p2 -> ... -> p6 -> salt: six cards, the most a dish may carry.
    let mut k = kitchen();
    for i in (1..=6).rev() {
        let below = if i == 6 { "salt".to_string() } else { format!("p{}", i + 1) };
        k.push((format!("p{i}"), format!(r#"{{"id":"p{i}","kind":"prep","card":{{"lines":[{{"item":"{below}","qty":1}}],"yield":1}}}}"#)));
    }
    assert_eq!(expand(&[("p1".into(), 1)], &lookup(&k)).unwrap()[0].item, "salt", "six levels expand");
    // A seventh on top is refused; so is one squeezed in below p6.
    let top = Card { lines: vec![Line { item: "p1".into(), qty: 1 }], yield_qty: 1 };
    assert!(matches!(check_card("p0", &top, &k), Err(Refusal::Tree(tree::Stop::TooDeep(_)))), "above");
    let bottom = Card { lines: vec![Line { item: "salt".into(), qty: 1 }], yield_qty: 1 };
    assert_eq!(check_card("p7", &bottom, &k), Ok(()), "a new leaf card alone is fine");
    k.push(("p7".into(), r#"{"id":"p7","kind":"prep","card":{"lines":[{"item":"salt","qty":1}],"yield":1}}"#.into()));
    let p6 = Card { lines: vec![Line { item: "p7".into(), qty: 1 }], yield_qty: 1 };
    assert!(matches!(check_card("p6", &p6, &k), Err(Refusal::Tree(tree::Stop::TooDeep(_)))), "p6 under five cards cannot take a card below");
}

#[test]
fn where_used_is_transitive_and_k_is_derived() {
    let k = kitchen();
    let products = vec![("philadelphia".to_string(), PHILADELPHIA.to_string()), ("plain".to_string(), r#"{"id":"plain","name":"Plain","bom":[{"supply":"salt","qty":3}]}"#.to_string())];
    let u = uses_of("salt", &k, &products);
    assert_eq!(u.preps, vec![("mitsukan".to_string(), "Mitsukan".to_string()), ("rice-seasoned".to_string(), "Rice seasoned".to_string())]);
    assert_eq!(u.dishes, vec![("philadelphia".to_string(), "Philadelphia".to_string()), ("plain".to_string(), "Plain".to_string())]);
    let u = uses_of("mitsukan", &k, &products);
    assert_eq!(u.preps, vec![("rice-seasoned".to_string(), "Rice seasoned".to_string())]);
    assert_eq!(u.dishes.len(), 1);
    assert_eq!(uses_of("nobody", &k, &products), Uses::default());
    // K: mitsukan 1000 / 1000 = 100 %; seasoned rice 2100 / 2350 = 89.4 %.
    let card = card_of(&lookup(&k)("rice-seasoned").unwrap()).unwrap();
    assert_eq!(k_pm(&card, "g", None, &lookup(&k)), Some(894));
    // An egg without a weight: K unknown, never a refusal.
    let mut k2 = k.clone();
    k2.push(("egg".into(), r#"{"id":"egg","unit":"unit","kind":"food_ingredient"}"#.into()));
    let sauce = Card { lines: vec![Line { item: "egg".into(), qty: 2 }, Line { item: "vinegar".into(), qty: 10 }], yield_qty: 100 };
    assert_eq!(k_pm(&sauce, "g", None, &lookup(&k2)), None);
    assert_eq!(check_card("sauce", &sauce, &k2), Ok(()));
    k2.push(("egg-w".into(), r#"{"id":"egg-w","unit":"unit","kind":"food_ingredient","weightPerUnit":55}"#.into()));
    let sauce = Card { lines: vec![Line { item: "egg-w".into(), qty: 2 }, Line { item: "vinegar".into(), qty: 10 }], yield_qty: 100 };
    assert_eq!(k_pm(&sauce, "g", None, &lookup(&k2)), Some(833), "100 / (110 + 10)");
}

#[test]
fn the_card_round_trips_and_a_raw_record_has_none() {
    let c = Card { lines: vec![Line { item: "salt".into(), qty: 50 }], yield_qty: 1000 };
    let rec = format!(r#"{{"id":"m","kind":"prep","card":{}}}"#, card_json(&c));
    assert_eq!(card_of(&rec), Some(c));
    // The catalogue spends a cell per byte: the fixed part, and Mitsukan whole.
    assert_eq!(card_json(&Card { lines: vec![], yield_qty: 1 }).to_string().len(), 22, "the card's fixed cost in bytes");
    let mitsukan = card_of(&kitchen().iter().find(|(i, _)| i == "mitsukan").unwrap().1).unwrap();
    assert_eq!(format!(r#""card":{}"#, card_json(&mitsukan)).len(), 112, "three lines and a yield");
    assert_eq!(card_of(r#"{"id":"salt","kind":"food_ingredient","card":{"lines":[],"yield":1}}"#), None, "a raw record's stray card is not one");
    assert!(!is_prep(r#"{"kind":"food_ingredient"}"#) && is_prep(r#"{"kind":"prep"}"#));
}
