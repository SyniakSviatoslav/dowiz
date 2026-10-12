//! W-NOM: an ingredient deleted from the catalogue leaves every recipe, and
//! each dish that used it is re-derived from what is left.
use super::*;
use crate::recipe::apply::set_bom;
use review::has_mark;

fn sup(id: &str, v: Value) -> (String, String) {
    (id.to_string(), v.to_string())
}

fn catalogue() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    for (id, j) in [
        sup("salmon", json!({ "id": "salmon", "name": "Salmon", "unit": "g", "kind": "food_ingredient", "kcalPer100": 208, "costPerBasis": 240 })),
        sup("rice", json!({ "id": "rice", "name": "Rice", "unit": "g", "kind": "food_ingredient", "kcalPer100": 130, "costPerBasis": 38 })),
    ] {
        cat.set_supply(&id, &j);
    }
    let lines = |v: &[(&str, i64)]| v.iter().map(|(s, q)| BomLineIn { supply: s.to_string(), qty: (*q).into(), net: None, out: None }).collect::<Vec<_>>();
    let mut roll = json!({ "id": "roll", "name": "Roll", "price": 650 });
    set_bom(&mut roll, &lines(&[("rice", 90), ("salmon", 35)]), |s| cat.supply(s), Typed::default()).unwrap();
    cat.set_product("roll", &roll.to_string());
    let mut sashimi = json!({ "id": "sashimi", "name": "Sashimi", "price": 900, "nutrition": { "kcal": 77 } });
    set_bom(&mut sashimi, &lines(&[("salmon", 100)]), |s| cat.supply(s), Typed { nutrition: true, ..Typed::default() }).unwrap();
    cat.set_product("sashimi", &sashimi.to_string());
    let mut plain = json!({ "id": "bowl", "name": "Bowl", "price": 300 });
    set_bom(&mut plain, &lines(&[("rice", 200)]), |s| cat.supply(s), Typed::default()).unwrap();
    cat.set_product("bowl", &plain.to_string());
    cat
}

fn product(cat: &Catalog, id: &str) -> Value {
    serde_json::from_str(&cat.product(id).unwrap()).unwrap()
}

#[test]
fn a_deleted_ingredient_leaves_the_catalogue_and_every_recipe_which_refolds() {
    let mut cat = catalogue();
    let bowl_before = cat.product("bowl");
    let r = remove_supplies(&mut cat, &["salmon".into(), "ghost".into()]);
    assert_eq!(r.deleted, vec!["salmon".to_string()], "only what the catalogue held");
    let mut dishes = r.dishes.clone();
    dishes.sort();
    assert_eq!(dishes, vec!["roll".to_string(), "sashimi".to_string()]);
    assert!(cat.supply("salmon").is_none());
    assert_eq!(cat.product("bowl"), bowl_before, "a dish without it is not rewritten");

    let roll = product(&cat, "roll");
    assert_eq!(roll["bom"], json!([{ "supply": "rice", "qty": 90 }]));
    assert_eq!(roll["nutrition"]["kcal"], json!(117), "90 g of rice alone");
    assert_eq!(roll["cost"], json!(34));
    assert_eq!(roll["weightG"], json!(90));
    assert_eq!(roll["ingredients"], json!(["Rice"]));
    // The stock ledger reads the recipe as it now is: salmon is not reserved.
    let bom = dowiz_hub::stock::bom_of(&roll.to_string());
    assert_eq!(bom.iter().map(|l| l.supply.as_str()).collect::<Vec<_>>(), vec!["rice"]);

    let sashimi = product(&cat, "sashimi");
    assert!(sashimi["bom"].is_null(), "its only line went: no recipe");
    assert_eq!(sashimi["nutrition"]["kcal"], json!(77), "what the owner TYPED stays");
    assert!(sashimi["cost"].is_null());

    // Twice: nothing more to delete, nothing rewritten.
    let again = remove_supplies(&mut cat, &["salmon".into()]);
    assert_eq!(again, Removal::default());
}

#[test]
fn ids_are_trimmed_distinct_and_bounded() {
    assert_eq!(ids_of(&[" a ".into(), "a".into(), "".into(), "b".into()]).unwrap(), vec!["a", "b"]);
    assert!(ids_of(&[" ".into()]).is_err());
    let max = crate::services::operations::stock::removed::MAX_IDS;
    let many: Vec<String> = (0..=max).map(|i| format!("s{i}")).collect();
    assert!(ids_of(&many).is_err());
    assert_eq!(ids_of(&many[..max]).unwrap().len(), max);
}

/// A recipe line whose supply was already gone keeps the stored lines minus
/// the deleted one, rather than refusing the whole deletion.
#[test]
fn a_recipe_with_a_stale_line_still_loses_the_deleted_one() {
    let mut cat = catalogue();
    cat.set_product("odd", &json!({ "id": "odd", "bom": [{ "supply": "lost", "qty": 5 }, { "supply": "rice", "qty": 10 }] }).to_string());
    remove_supplies(&mut cat, &["rice".into()]);
    assert_eq!(product(&cat, "odd")["bom"], json!([{ "supply": "lost", "qty": 5 }]));
}

/// SPEC-SEMI-FINISHED §e: the where-used answer names every card and dish an
/// item reaches (RED before 2026-09-29: `uses_of` did not exist and the
/// route removed lines with nobody asked), and a deletion strips the item
/// from every card too.
#[test]
fn where_used_names_cards_and_dishes_and_a_deletion_strips_the_cards() {
    let mut cat = catalogue();
    cat.set_supply("mitsukan", &json!({ "id": "mitsukan", "name": "Mitsukan", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "rice", "qty": 50 }, { "item": "salmon", "qty": 5 }], "yield": 100 } }).to_string());
    let mut dish = json!({ "id": "pf-dish", "name": "PF dish", "price": 500 });
    set_bom(&mut dish, &[BomLineIn { supply: "mitsukan".into(), qty: 20.into(), net: None, out: None }], |s| cat.supply(s), Typed::default()).unwrap();
    cat.set_product("pf-dish", &dish.to_string());
    let u = uses_of(&cat, &["salmon".into(), "nobody".into()]);
    let salmon = &u["salmon"];
    assert_eq!(salmon["preps"], json!([{ "id": "mitsukan", "name": "Mitsukan" }]));
    let dishes: Vec<&str> = salmon["dishes"].as_array().unwrap().iter().map(|d| d["id"].as_str().unwrap()).collect();
    assert_eq!(dishes, vec!["pf-dish", "roll", "sashimi"], "through the card AND directly");
    assert!(u.get("nobody").is_none(), "an unused id is absent, so an empty map means nothing is used");
    assert!(uses_of(&cat, &["ghost".into()]).as_object().unwrap().is_empty());

    let r = remove_supplies(&mut cat, &["salmon".into()]);
    assert!(r.dishes.contains(&"pf-dish".to_string()), "the dish through the card is re-derived: {:?}", r.dishes);
    let card = dowiz_hub::prep::card_of(&cat.supply("mitsukan").unwrap()).unwrap();
    assert_eq!(card.lines.iter().map(|l| l.item.as_str()).collect::<Vec<_>>(), vec!["rice"], "the card lost the line");
    assert_eq!(card.yield_qty, 100, "and kept its yield");
    assert_eq!(product(&cat, "pf-dish")["bom"], json!([{ "supply": "mitsukan", "qty": 20 }]), "the dish still names the card");
    assert_eq!(product(&cat, "pf-dish")["cost"], json!(4), "20 g at 50 g rice per 100 g: 10 g rice at 38/100 = 3.8");
}

/// Live on qa-durres 2026-09-29: deleting a ПФ TOGETHER WITH an item on its
/// card answered `deleted` for both, then the card rewrite (which walked the
/// catalogue as it was before) stripped the line and wrote the deleted ПФ
/// straight back.
#[test]
fn a_prep_deleted_with_its_own_line_stays_deleted() {
    let mut cat = catalogue();
    cat.set_supply("mitsukan", &json!({ "id": "mitsukan", "name": "Mitsukan", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "rice", "qty": 50 }, { "item": "salmon", "qty": 5 }], "yield": 100 } }).to_string());
    let r = remove_supplies(&mut cat, &["mitsukan".into(), "salmon".into()]);
    assert!(r.deleted.contains(&"mitsukan".to_string()));
    assert!(cat.supply("mitsukan").is_none(), "the deleted ПФ came back");
    assert!(cat.supply("salmon").is_none());
    // The twin: a ПФ that is NOT deleted still loses the deleted line and stays.
    let mut cat = catalogue();
    cat.set_supply("mitsukan", &json!({ "id": "mitsukan", "name": "Mitsukan", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "rice", "qty": 50 }, { "item": "salmon", "qty": 5 }], "yield": 100 } }).to_string());
    remove_supplies(&mut cat, &["salmon".into()]);
    assert!(cat.supply("mitsukan").is_some(), "a kept ПФ stays");
}

/// W-VERIFY 2026-09-30: the ONLY line of a ПФ card deleted (confirmed) left
/// the ПФ with `lines: []` -- a card `check_card` refuses on save. Every dish
/// through it then expanded to NO leaf: it sold drawing nothing and its cost
/// fell to 0, silently. Refused now; deleting the ПФ in the same call is fine.
#[test]
fn a_delete_that_would_empty_a_card_is_refused_even_when_confirmed() {
    let mut cat = catalogue();
    cat.set_supply("brine", &json!({ "id": "brine", "name": "Brine", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "salmon", "qty": 50 }], "yield": 100 } }).to_string());
    let before = cat.supply("brine");
    assert_eq!(delete_in(&mut cat, &["salmon".into()], true).map(|_| ()), Err(Held::Emptied(vec!["brine".into()])));
    assert_eq!(cat.supply("brine"), before, "nothing changed");
    assert!(cat.supply("salmon").is_some(), "nothing changed");
    // Twins: the ПФ deleted with its line, and a card that keeps a line.
    assert!(delete_in(&mut cat, &["salmon".into(), "brine".into()], true).is_ok());
    assert!(cat.supply("brine").is_none() && cat.supply("salmon").is_none());
    let mut cat = catalogue();
    cat.set_supply("brine", &json!({ "id": "brine", "name": "Brine", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "salmon", "qty": 50 }, { "item": "rice", "qty": 5 }], "yield": 100 } }).to_string());
    assert!(delete_in(&mut cat, &["salmon".into()], true).is_ok());
    assert_eq!(dowiz_hub::prep::card_of(&cat.supply("brine").unwrap()).unwrap().lines.len(), 1);
    // Unconfirmed, the where-used answer still comes first.
    let mut cat = catalogue();
    assert!(matches!(delete_in(&mut cat, &["salmon".into()], false), Err(Held::InUse(_))));
}

/// TZ 2026-10-12 item 3 (contract §2): a deleted ingredient MARKS every dish
/// and card that used it, naming it; a re-derivation keeps the mark; only
/// the next hand-saved recipe clears it. RED before W-QTY: no mark existed.
#[test]
fn a_deletion_marks_each_dish_and_card_for_review_until_the_recipe_is_saved_by_hand() {
    use crate::services::operations::preps::{self, LineIn, PrepIn};
    let mut cat = catalogue();
    // A card under a dish: roll-pf = sushi-rice (rice + salmon) + rice.
    let body = PrepIn { id: "sushi-rice".into(), name: Some("Sushi rice".into()), lines: vec![LineIn { item: "rice".into(), qty: 100 }, LineIn { item: "salmon".into(), qty: 5 }], yield_qty: 100, ..PrepIn::default() };
    let (pid, card) = preps::check(&body).unwrap();
    preps::save(&mut cat, &pid, &body, &card).unwrap();
    let mut d = json!({ "id": "roll-pf", "name": "Roll PF", "price": 700 });
    set_bom(&mut d, &[BomLineIn { supply: "sushi-rice".into(), qty: 50.into(), net: None, out: None }, BomLineIn { supply: "rice".into(), qty: 10.into(), net: None, out: None }], |s| cat.supply(s), Typed::default()).unwrap();
    cat.set_product("roll-pf", &d.to_string());

    let r = remove_supplies(&mut cat, &["salmon".into()]);
    // The dish that named it: marked with its name and quantity; the line is gone from the ledger's view.
    let roll = product(&cat, "roll");
    assert_eq!(roll["recipeReview"], json!([{ "supply": "salmon", "name": "Salmon", "qty": 35 }]));
    assert_eq!(roll["bom"], json!([{ "supply": "rice", "qty": 90 }]));
    // The card that named it: marked on the card, lines without it; the dish above it marked `via`.
    let pf = serde_json::from_str::<Value>(&cat.supply("sushi-rice").unwrap()).unwrap();
    assert_eq!(pf["card"]["lines"], json!([{ "item": "rice", "qty": 100 }]));
    assert_eq!(pf["review"], json!([{ "supply": "salmon", "name": "Salmon", "qty": 5 }]));
    assert_eq!(dowiz_hub::minijson::str_field(&pf.to_string(), "name").as_deref(), Some("Sushi rice"), "the mark never shadows the ПФ's own name");
    assert_eq!(product(&cat, "roll-pf")["recipeReview"], json!([{ "supply": "salmon", "name": "Salmon", "qty": 5, "via": "sushi-rice" }]));
    assert!(product(&cat, "bowl").get("recipeReview").is_none(), "a dish that never used it is not marked");
    // The answer names every mark; the where-used rows say so; the ПФ list shows it.
    let ids: Vec<&str> = r.review.iter().map(|m| m["id"].as_str().unwrap()).collect();
    assert_eq!(ids, vec!["sushi-rice", "roll", "roll-pf", "sashimi"]);
    assert!(r.review[0]["prep"] == json!(true));
    let u = preps::answer(&cat, "uses:rice").unwrap();
    assert_eq!(u["uses"]["dishes"].as_array().unwrap().iter().map(|d| (d["id"].as_str().unwrap(), d["review"] == json!(true))).collect::<Vec<_>>(), vec![("bowl", false), ("roll", true), ("roll-pf", true)]);
    assert_eq!(u["uses"]["preps"], json!([{ "id": "sushi-rice", "name": "Sushi rice", "review": true }]));
    let listed = preps::list(&cat);
    assert_eq!(listed[0]["review"], json!([{ "supply": "salmon", "name": "Salmon", "qty": 5 }]));
    assert!(has_mark(&cat.supply("sushi-rice").unwrap()) && has_mark(&cat.product("roll").unwrap()) && !has_mark(&cat.product("bowl").unwrap()));
    // The owner's read carries the mark; a price change re-derives and KEEPS it.
    let mut read = product(&cat, "roll");
    crate::recipe::hydrate(&mut read, |s| cat.supply(s));
    assert_eq!(read["recipeReview"][0]["name"], json!("Salmon"));
    cat.set_supply("rice", &json!({ "id": "rice", "name": "Rice", "unit": "g", "kind": "food_ingredient", "kcalPer100": 130, "costPerBasis": 50 }).to_string());
    assert!(crate::services::operations::preps::rederive::dishes_using(&mut cat, "rice").contains(&"roll".to_string()));
    assert_eq!(product(&cat, "roll")["cost"], json!(45));
    assert_eq!(product(&cat, "roll")["recipeReview"][0]["supply"], json!("salmon"), "a re-derivation is not a review");
    // The owner saves the recipe by hand: the mark goes. A ghost line sent back is refused by name.
    let mut roll = product(&cat, "roll");
    let err = set_bom(&mut roll, &[BomLineIn { supply: "salmon".into(), qty: 35.into(), net: None, out: None }], |s| cat.supply(s), Typed::default()).unwrap_err();
    assert_eq!(err, "unknown supply salmon");
    set_bom(&mut roll, &[BomLineIn { supply: "rice".into(), qty: 90.into(), net: None, out: None }], |s| cat.supply(s), Typed::default()).unwrap();
    assert!(roll.get("recipeReview").is_none(), "saved by hand: reviewed");
    // The card saved by hand (its lines without the ghost) drops its mark too.
    let body = PrepIn { id: "sushi-rice".into(), lines: vec![LineIn { item: "rice".into(), qty: 100 }], yield_qty: 100, ..PrepIn::default() };
    let (pid, card) = preps::check(&body).unwrap();
    preps::save(&mut cat, &pid, &body, &card).unwrap();
    assert!(!has_mark(&cat.supply("sushi-rice").unwrap()));
}
