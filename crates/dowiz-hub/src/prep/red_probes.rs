//! W-QTYHUB RED probes for H1/H2, on the API main already had.

use super::tests::{kitchen, PHILADELPHIA};
use super::*;

fn lookup(list: &[(String, String)]) -> impl Fn(&str) -> Option<String> + '_ {
    move |id| list.iter().find(|(i, _)| i == id).map(|(_, j)| j.clone())
}

/// H1: half a sheet of nori beside a semi-finished line stays half a sheet.
#[test]
fn red_h1_for_ledger_keeps_a_fractional_raw_line() {
    let mut k = kitchen();
    k.push(("nori".into(), r#"{"id":"nori","name":"Nori","unit":"unit","kind":"food_ingredient"}"#.into()));
    let dish = PHILADELPHIA.replace(r#"{"supply":"rice-seasoned","qty":130}"#, r#"{"supply":"rice-seasoned","qty":130},{"supply":"nori","uq":500000}"#);
    let (j, _) = for_ledger(&lookup(&k), &dish);
    let nori = crate::stock::bom_of(&j).into_iter().find(|l| l.supply == "nori").expect("the nori leaf");
    assert_eq!(nori.uq, 500_000, "half a sheet, exactly");
}

/// H2: a card line of half a gram (`uq`) and a fractional yield are read.
#[test]
fn red_h2_a_card_reads_half_a_gram() {
    let mut k = kitchen();
    k.push(("saffron".into(), r#"{"id":"saffron","name":"Saffron","unit":"g","kind":"spice"}"#.into()));
    k.push(("sauce".into(), r#"{"id":"sauce","name":"Sauce","unit":"g","kind":"prep","card":{"lines":[{"item":"saffron","uq":500000}],"yieldUq":999500000}}"#.into()));
    let leaves = expand(&[("sauce".into(), 1999)], &lookup(&k)).unwrap();
    assert_eq!(leaves.iter().map(|l| (l.item.as_str(), l.uq)).collect::<Vec<_>>(), vec![("saffron", 1_000_000)]);
}
