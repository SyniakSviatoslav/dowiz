//! W-QTYHUB H1 + H2 (CONTRACT-QTY §5): a dish's fractional raw line beside a
//! semi-finished one expands EXACTLY (H1), and a card carries half a gram and
//! a fractional yield in millionths (H2) -- while every whole card keeps the
//! bytes it always had.

use super::tests::{kitchen, PHILADELPHIA};
use super::*;

fn lookup(list: &[(String, String)]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| list.iter().find(|(i, _)| i == id).map(|(_, j)| j.clone())
}

fn with_saffron() -> Vec<(String, String)> {
    let mut k = kitchen();
    k.push(("saffron".into(), r#"{"id":"saffron","name":"Saffron","unit":"g","kind":"spice"}"#.into()));
    k.push(("cream".into(), r#"{"id":"cream","name":"Cream","unit":"ml","kind":"food_ingredient"}"#.into()));
    // Half a gram of saffron in a batch of 999.5 g of sauce.
    k.push(("sauce".into(), r#"{"id":"sauce","name":"Sauce","unit":"g","kind":"prep","card":{"lines":[{"item":"saffron","uq":500000},{"item":"cream","qty":1000}],"yieldUq":999500000}}"#.into()));
    k
}

/// H1: `for_ledger` reads `uq`, so 0.5 nori stays 0.5 -- not the 1 whole
/// sheet `bom_of`'s rounded `qty` would book.
#[test]
fn a_fractional_raw_line_beside_a_semi_finished_one_expands_exactly() {
    let mut k = kitchen();
    k.push(("nori".into(), r#"{"id":"nori","name":"Nori","unit":"unit","kind":"food_ingredient"}"#.into()));
    let dish = PHILADELPHIA.replace(r#"{"supply":"rice-seasoned","qty":130}"#, r#"{"supply":"rice-seasoned","qty":130},{"supply":"nori","uq":500000}"#);
    let (j, why) = for_ledger(&lookup(&k), &dish);
    assert_eq!(why, None);
    let nori = crate::stock::bom_of(&j).into_iter().find(|l| l.supply == "nori").expect("the nori leaf");
    assert_eq!(nori.uq, 500_000, "half a sheet, exactly");
    // The tree the shelf walks carries the same half (`stock::basket`).
    let b = crate::stock::basket::of(&[(j, 2)]).expect("a tree");
    let root = b.roots.iter().find(|(s, _)| s == "nori").expect("nori root").1;
    assert_eq!(root, crate::import::recipes::num::Rat::new(1, 1), "two portions of half a sheet");
    // A whole raw line beside it writes the old form into the tree.
    let (j, _) = for_ledger(&lookup(&k), PHILADELPHIA);
    assert!(j.contains(r#""lines":[{"qty":130,"supply":"rice-seasoned"}]"#) || j.contains(r#"{"supply":"rice-seasoned","qty":130}"#), "{j}");
}

/// H2: a card line in `uq` and a `yieldUq` are read, expanded, checked and
/// written back as they were stored; a whole card is byte for byte.
#[test]
fn a_card_carries_half_a_gram_and_a_fractional_yield() {
    let k = with_saffron();
    let card = card_of(&lookup(&k)("sauce").unwrap()).expect("the card");
    assert_eq!(card, Card { lines: vec![Line { item: "saffron".into(), uq: 500_000 }, Line::whole("cream", 1000)], yield_uq: 999_500_000 });
    assert_eq!(card_json(&card).to_string(), r#"{"lines":[{"item":"saffron","uq":500000},{"item":"cream","qty":1000}],"yieldUq":999500000}"#);
    assert_eq!(check_card("sauce", &card, &k), Ok(()), "half a gram is a quantity");
    // 199.9 g of sauce is a fifth of the batch: 0.1 g of saffron, 200 ml of cream.
    let leaves = expand_micro(&[("sauce".into(), 199_900_000)], &lookup(&k)).unwrap();
    let got: Vec<(&str, i64)> = leaves.iter().map(|l| (l.item.as_str(), l.uq)).collect();
    assert_eq!(got, vec![("cream", 200_000_000), ("saffron", 100_000)]);
    // The whole card of the operator's example writes its old bytes.
    let mitsukan = card_of(&lookup(&k)("mitsukan").unwrap()).unwrap();
    assert_eq!(card_json(&mitsukan).to_string(), r#"{"lines":[{"item":"vinegar","qty":800},{"item":"salt","qty":50},{"item":"sugar","qty":150}],"yield":1000}"#);
    assert_eq!((mitsukan.lines[1].qty(), mitsukan.yield_qty()), (50, 1000));
    // Zero is still not a quantity.
    let zero = Card { lines: vec![Line { item: "saffron".into(), uq: 0 }], yield_uq: MICRO };
    assert!(matches!(check_card("sauce", &zero, &k), Err(Refusal::Bound(_))));
}

/// H2 through the shelf: a sale of the dish reaches the fractional card line
/// through the basket's tree.
#[test]
fn the_basket_reads_a_fractional_card_line() {
    let k = with_saffron();
    let dish = r#"{"id":"pasta","bom":[{"supply":"sauce","qty":1999}]}"#;
    let (j, why) = for_ledger(&lookup(&k), dish);
    assert_eq!(why, None);
    let saffron = crate::stock::bom_of(&j).into_iter().find(|l| l.supply == "saffron").expect("saffron leaf");
    assert_eq!(saffron.uq, 1_000_000, "1999 g of a 999.5 g batch: two batches, one gram of saffron");
    let b = crate::stock::basket::of(&[(j, 1)]).expect("a tree");
    let sauce = &b.cards["sauce"];
    assert_eq!(sauce.lines[0].qty, crate::import::recipes::num::Rat::new(1, 2));
    assert_eq!(sauce.batch, crate::import::recipes::num::Rat::new(1999, 2));
}
