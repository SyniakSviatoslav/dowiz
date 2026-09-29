//! The operator's example through the routes' pure halves: a card saved,
//! checked and re-deriving its dishes; the where-used answer; one sale's
//! leaves; every refusal beside its twin.

use super::*;
use crate::recipe::apply::{set_bom, Typed};
use crate::recipe::BomLineIn;

fn raw(id: &str, unit: &str, cost: i64, kcal: f64) -> (String, String) {
    (id.into(), json!({ "id": id, "name": id, "unit": unit, "kind": "food_ingredient", "costPerBasis": cost, "kcalPer100": kcal }).to_string())
}

/// Raw prices in lek per 100 g/ml: rice 20, vinegar 15, salt 5, sugar 12.
pub(crate) fn kitchen() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    for (id, j) in [raw("rice-dry", "g", 20, 350.0), raw("vinegar", "ml", 15, 20.0), raw("salt", "g", 5, 0.0), raw("sugar", "g", 12, 400.0)] {
        cat.set_supply(&id, &j);
    }
    cat.set_supply("water", &json!({ "id": "water", "name": "Water", "unit": "ml", "kind": "food_ingredient", "untracked": true, "costPerBasis": 0, "kcalPer100": 0 }).to_string());
    cat
}

fn prep_in(id: &str, lines: &[(&str, i64)], yield_qty: i64) -> PrepIn {
    PrepIn { id: id.into(), name: Some(id.into()), unit: Some("g".into()), lines: lines.iter().map(|(i, q)| LineIn { item: (*i).into(), qty: *q }).collect(), yield_qty, ..PrepIn::default() }
}

fn saved(cat: &mut Catalog, body: &PrepIn) -> std::result::Result<(Value, Vec<String>), String> {
    let (id, card) = check(body)?;
    save(cat, &id, body, &card)
}

fn product(cat: &Catalog, id: &str) -> Value {
    serde_json::from_str(&cat.product(id).unwrap()).unwrap()
}

#[test]
fn the_example_is_saved_costed_and_the_dish_follows_a_price_change_unsaved() {
    let mut cat = kitchen();
    saved(&mut cat, &prep_in("mitsukan", &[("vinegar", 800), ("salt", 50), ("sugar", 150)], 1000)).unwrap();
    let (rice, _) = saved(&mut cat, &prep_in("rice-seasoned", &[("rice-dry", 1000), ("water", 1100), ("mitsukan", 250)], 2100)).unwrap();
    assert_eq!(rice["kind"], json!("prep"));
    assert_eq!(rice["card"]["yield"], json!(2100));
    // The dish: 130 g of it. Stored cost 14.555 lek -> 15 (lek has no minor unit here).
    let mut dish = json!({ "id": "philadelphia", "name": "Philadelphia", "price": 650 });
    set_bom(&mut dish, &[BomLineIn { supply: "rice-seasoned".into(), qty: 130, net: None, out: None }], |s| cat.supply(s), Typed::default()).unwrap();
    assert_eq!(dish["bom"], json!([{ "supply": "rice-seasoned", "qty": 130 }]), "stored lean, naming the ПФ");
    assert_eq!(dish["cost"], json!(15));
    assert_eq!(dish["weightG"], json!(130), "a ПФ line weighs what it says: no losses");
    cat.set_product("philadelphia", &dish.to_string());
    // The view: K, cost per kg, per line.
    let v = list(&cat);
    let rs = v.iter().find(|p| p["id"] == "rice-seasoned").unwrap();
    assert_eq!(rs["k"], json!(894));
    assert_eq!(rs["costPer"], json!(112), "111.96 lek per kg");
    assert_eq!(rs["batchCost"], json!(235));
    assert_eq!(rs["uses"]["dishes"][0]["id"], json!("philadelphia"));
    let m = v.iter().find(|p| p["id"] == "mitsukan").unwrap();
    assert_eq!(m["uses"]["preps"][0]["id"], json!("rice-seasoned"));
    assert_eq!(m["lines"][0]["cost"], json!(120), "800 ml of vinegar at 15/100");
    // The dry rice goes to 30 per 100 g: the OWNER's read re-derives, nothing saved...
    cat.set_supply("rice-dry", &raw("rice-dry", "g", 30, 350.0).1);
    let mut read = product(&cat, "philadelphia");
    crate::recipe::hydrate(&mut read, |s| cat.supply(s));
    assert_eq!(read["cost"], json!(21), "20.75 lek");
    assert_eq!(product(&cat, "philadelphia")["cost"], json!(15), "the stored copy until a write re-derives it");
    // ... and the supply write's re-derivation catches the stored copy up.
    assert_eq!(rederive::dishes_using(&mut cat, "rice-dry"), vec!["philadelphia".to_string()]);
    assert_eq!(product(&cat, "philadelphia")["cost"], json!(21));
    // One sale's leaves.
    let t = takes(&cat, &cat.product("philadelphia").unwrap());
    let rows: Vec<(String, String)> = t["leaves"].as_array().unwrap().iter().map(|l| (l["supply"].as_str().unwrap().into(), l["qty"].as_str().unwrap().into())).collect();
    assert_eq!(rows, vec![("rice-dry".into(), "61.905".into()), ("salt".into(), "0.774".into()), ("sugar".into(), "2.321".into()), ("vinegar".into(), "12.381".into())]);
    assert_eq!(t["cost"], json!(21));
}

#[test]
fn a_cycle_is_refused_with_its_path_and_the_catalogue_is_untouched() {
    let mut cat = kitchen();
    saved(&mut cat, &prep_in("a", &[("salt", 1)], 1)).unwrap();
    saved(&mut cat, &prep_in("b", &[("a", 1)], 1)).unwrap();
    let before = cat.supply("a").unwrap();
    let why = saved(&mut cat, &prep_in("a", &[("b", 1)], 1)).unwrap_err();
    assert!(why.contains("a → b → a"), "{why}");
    assert_eq!(cat.supply("a").unwrap(), before, "nothing written");
    assert_eq!(saved(&mut cat, &prep_in("a", &[("a", 1)], 1)).unwrap_err(), "a semi-finished product cannot contain itself");
    // Twins: a new card under b passes; a raw id cannot become a card.
    saved(&mut cat, &prep_in("c", &[("b", 2), ("sugar", 3)], 5)).unwrap();
    assert!(saved(&mut cat, &prep_in("salt", &[("sugar", 1)], 1)).unwrap_err().contains("raw ingredient"));
    assert!(saved(&mut cat, &prep_in("d", &[("ghost", 1)], 1)).unwrap_err().contains("unknown supply ghost"));
}

#[test]
fn the_body_is_checked_before_the_catalogue_is_read() {
    let mut b = prep_in("", &[("salt", 1)], 1);
    assert!(check(&b).is_err());
    b.id = "x".into();
    b.unit = Some("kg".into());
    assert_eq!(check(&b).unwrap_err(), "unit is g, ml or unit");
    b.unit = Some("ml".into());
    b.weight_per_unit = Some(-1.0);
    assert!(check(&b).is_err());
    b.weight_per_unit = None;
    let (id, card) = check(&b).unwrap();
    assert_eq!((id.as_str(), card.yield_qty, card.lines[0].item.as_str()), ("x", 1, "salt"));
    // The record keeps what the body did not say and drops the nulls.
    let r = record("x", &b, &card, &json!({ "category": "Sauces", "weightPerUnit": 12 }));
    assert_eq!((r["category"].clone(), r["weightPerUnit"].clone(), r["active"].clone()), (json!("Sauces"), json!(12), json!(true)));
    assert_eq!(r["kind"], json!("prep"));
    let r = record("x", &PrepIn { active: Some(false), ..b.clone() }, &card, &json!({}));
    assert_eq!(r["active"], json!(false));
    assert!(r.get("category").is_some_and(|c| c == ""));
}

#[test]
fn a_card_edit_re_derives_the_dishes_that_use_it() {
    let mut cat = kitchen();
    saved(&mut cat, &prep_in("mitsukan", &[("vinegar", 800), ("salt", 50), ("sugar", 150)], 1000)).unwrap();
    let mut dish = json!({ "id": "d", "price": 100 });
    set_bom(&mut dish, &[BomLineIn { supply: "mitsukan".into(), qty: 100, net: None, out: None }], |s| cat.supply(s), Typed::default()).unwrap();
    assert_eq!(dish["cost"], json!(14), "14.05 lek");
    cat.set_product("d", &dish.to_string());
    // Twice the sugar: the batch costs more per gram, the dish follows in the same write.
    let (_, dishes) = saved(&mut cat, &prep_in("mitsukan", &[("vinegar", 800), ("salt", 50), ("sugar", 300)], 1000)).unwrap();
    assert_eq!(dishes, vec!["d".to_string()]);
    assert_eq!(product(&cat, "d")["cost"], json!(16), "(120 + 2.5 + 36) / 10");
    // A dish not using it is not rewritten.
    cat.set_product("plain", &json!({ "id": "plain", "bom": [{ "supply": "salt", "qty": 3 }] }).to_string());
    let before = cat.product("plain").unwrap();
    saved(&mut cat, &prep_in("mitsukan", &[("vinegar", 800), ("salt", 50), ("sugar", 150)], 1000)).unwrap();
    assert_eq!(cat.product("plain").unwrap(), before);
}

#[test]
fn a_dish_that_cannot_expand_says_why_instead_of_a_partial_list() {
    let cat = kitchen();
    let t = takes(&cat, r#"{"id":"x","bom":[{"supply":"nobody","qty":1}]}"#);
    assert_eq!(t["leaves"], json!([]));
    assert!(t["refused"].as_str().unwrap().contains("nobody"));
    let none = takes(&cat, r#"{"id":"x"}"#);
    assert_eq!((none["leaves"].clone(), none["lines"].clone()), (json!([]), json!(0)));
}
