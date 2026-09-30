//! W-VERIFY (2026-09-30): the edges of the ПФ arithmetic the lane's own tests
//! did not reach -- a piece in a gram card, depth 6 and 7 from a DISH, a
//! diamond, the bounds, K > 1, water only, an empty card, and the i128
//! headroom at the bounds. Every case calls the real functions.

use super::tests::kitchen;
use super::*;

fn lookup(list: &[(String, String)]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| list.iter().find(|(i, _)| i == id).map(|(_, j)| j.clone())
}
fn prep(id: &str, unit: &str, lines: &[(&str, i64)], yield_qty: i64) -> (String, String) {
    let ls: Vec<String> = lines.iter().map(|(i, q)| format!(r#"{{"item":"{i}","qty":{q}}}"#)).collect();
    (id.to_string(), format!(r#"{{"id":"{id}","name":"{id}","unit":"{unit}","kind":"prep","card":{{"lines":[{}],"yield":{yield_qty}}}}}"#, ls.join(",")))
}
fn uq(leaves: &[Leaf], item: &str) -> Option<i64> {
    leaves.iter().find(|l| l.item == item).map(|l| l.uq)
}

/// A piece line (an egg, 55 g) in a gram card: the write-off counts EGGS, not
/// grams -- the line's quantity is in the item's own unit. K needs the weight.
#[test]
fn a_piece_in_a_gram_card_draws_pieces_and_k_needs_its_weight() {
    let mut k = kitchen();
    k.push(("egg".into(), r#"{"id":"egg","unit":"unit","kind":"food_ingredient","weightPerUnit":55}"#.into()));
    k.push(prep("mayo", "g", &[("egg", 2), ("vinegar", 10)], 100));
    let l = expand(&[("mayo".into(), 25)], &lookup(&k)).unwrap();
    assert_eq!(uq(&l, "egg"), Some(500_000), "25 g of 100 g made from 2 eggs = half an egg");
    assert_eq!(uq(&l, "vinegar"), Some(2_500_000));
    let card = card_of(&lookup(&k)("mayo").unwrap()).unwrap();
    assert_eq!(k_pm(&card, "g", None, &lookup(&k)), Some(833));
    // The ПФ itself counted in pieces: K needs ITS weight too.
    assert_eq!(k_pm(&card, "unit", None, &lookup(&k)), None);
    // 100 pieces of 1.2 g = 120 g out of 110 + 10 g in: K 100 %.
    assert_eq!(k_pm(&card, "unit", Some(1.2), &lookup(&k)), Some(1000));
}

/// dish -> p1 -> ... -> p6 -> salt expands (six cards, SPEC §b); a seventh
/// card that got into the catalogue anyway is refused LOUDLY by the walk, and
/// for_ledger hands that refusal back.
#[test]
fn depth_six_from_a_dish_expands_and_seven_is_refused() {
    let mut k = kitchen();
    for i in (1..=7).rev() {
        let below = if i == 7 { "salt".to_string() } else { format!("p{}", i + 1) };
        k.push(prep(&format!("p{i}"), "g", &[(below.as_str(), 2)], 1));
    }
    let six = expand(&[("p2".into(), 1)], &lookup(&k)).unwrap();
    assert_eq!(six, vec![Leaf { item: "salt".into(), uq: 64 * MICRO }], "2^6");
    assert!(matches!(expand(&[("p1".into(), 1)], &lookup(&k)), Err(Refusal::Tree(tree::Stop::TooDeep(_)))));
    let dish = r#"{"id":"deep","bom":[{"supply":"p1","qty":1}]}"#;
    let (same, why) = for_ledger(&lookup(&k), dish);
    assert_eq!(same, dish);
    assert!(matches!(why, Some(Refusal::Tree(tree::Stop::TooDeep(_)))));
}

/// One raw item reached by two paths is summed EXACTLY and rounded once:
/// 1/3 + 1/3 of a gram is 666 667 millionths, not 333 333 + 333 333.
#[test]
fn a_diamond_sums_the_paths_before_the_one_rounding() {
    let mut k = kitchen();
    k.push(prep("a", "g", &[("salt", 1)], 3));
    k.push(prep("b", "g", &[("salt", 1)], 3));
    k.push(prep("top", "g", &[("a", 1), ("b", 1)], 1));
    let l = expand(&[("top".into(), 1)], &lookup(&k)).unwrap();
    assert_eq!(l, vec![Leaf { item: "salt".into(), uq: 666_667 }]);
    // The walk reports two uses of the leaf.
    let w = leaves_exact("dish", &[("top".into(), 1)], &lookup(&k)).unwrap();
    assert_eq!(w.leaves[0].uses, 2);
}

/// Bounds: a zero or negative dish line draws nothing (and is not an error
/// here -- `recipe` refuses such a line on save); a yield of 0 in a stored
/// card is a loud refusal, never a division.
#[test]
fn zero_negative_and_no_yield() {
    let mut k = kitchen();
    assert_eq!(expand(&[("rice-seasoned".into(), 0)], &lookup(&k)).unwrap(), vec![]);
    assert_eq!(expand(&[("rice-seasoned".into(), -130)], &lookup(&k)).unwrap(), vec![]);
    k.push(prep("nobatch", "g", &[("salt", 1)], 0));
    assert_eq!(expand(&[("nobatch".into(), 1)], &lookup(&k)), Err(Refusal::Tree(tree::Stop::NoBatch("nobatch".into()))));
    k.push(prep("negbatch", "g", &[("salt", 1)], -5));
    assert!(expand(&[("negbatch".into(), 1)], &lookup(&k)).is_err());
    assert!(matches!(check_card("x", &Card { lines: vec![Line { item: "salt".into(), qty: -1 }], yield_qty: 1 }, &k), Err(Refusal::Bound(_))));
    assert!(matches!(check_card("x", &Card { lines: vec![Line { item: "salt".into(), qty: 1 }], yield_qty: 0 }, &k), Err(Refusal::Bound(_))));
}

/// Yield above gross (rice swells, K > 1) is legal: the leaves shrink.
#[test]
fn a_yield_above_its_gross_is_k_above_one() {
    let mut k = kitchen();
    k.push(prep("boiled", "g", &[("rice-dry", 1000)], 2500));
    assert_eq!(check_card("boiled2", &Card { lines: vec![Line { item: "rice-dry".into(), qty: 1000 }], yield_qty: 2500 }, &k), Ok(()));
    let card = card_of(&lookup(&k)("boiled").unwrap()).unwrap();
    assert_eq!(k_pm(&card, "g", None, &lookup(&k)), Some(2500));
    assert_eq!(expand(&[("boiled".into(), 250)], &lookup(&k)).unwrap(), vec![Leaf { item: "rice-dry".into(), uq: 100 * MICRO }]);
}

/// A card of water only draws nothing, and its cost is 0 -- water has no
/// price and is untracked by design.
#[test]
fn water_only_draws_nothing_and_costs_nothing() {
    let mut k = kitchen();
    k.push(prep("stock", "ml", &[("water", 1000)], 900));
    assert_eq!(expand(&[("stock".into(), 100)], &lookup(&k)).unwrap(), vec![]);
    assert_eq!(cost_micro(&[("stock".into(), 1)], &lookup(&k), &|_| None), Ok(Some(0)));
}

/// THE HEADROOM AT THE BOUNDS (SPEC §b says 10^41 in the worst case and
/// overflow is a loud refusal). A card of six levels, every line and yield at
/// coprime values near QTY_MAX, SAVES (check_card walks without the micro
/// scaling) and then cannot be SOLD: `expand` overflows at `to_micro`.
#[test]
fn a_card_that_saves_at_the_bounds_can_still_be_expanded() {
    let mut k = kitchen();
    let (q, y) = (99_991, 99_989); // both prime
    for i in (1..=6).rev() {
        let below = if i == 6 { "salt".to_string() } else { format!("h{}", i + 1) };
        k.push(prep(&format!("h{i}"), "g", &[(below.as_str(), q)], y));
    }
    // h1 was accepted by the save door:
    let h1 = card_of(&lookup(&k)("h1").unwrap()).unwrap();
    let rest: Vec<(String, String)> = k.iter().filter(|(i, _)| i != "h1").cloned().collect();
    assert_eq!(check_card("h1", &h1, &rest), Ok(()));
    // ...and one portion of 100 000 g of it must expand (or the save must refuse).
    let r = expand(&[("h1".into(), QTY_MAX)], &lookup(&k));
    assert!(r.is_ok(), "saved, but a sale of it is {r:?}");
}
