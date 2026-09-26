//! I0c: one tap (or one category) links a dish with no recipe to its own
//! piece, and a sale then takes one piece off the shelf.

use super::*;

fn catalog() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    cat.set_category("drinks", r#"{"id":"drinks","name":"Drinks"}"#);
    cat.set_product("cola", r#"{"id":"cola","name":"Cola 0.33","categoryId":"drinks","price":200}"#);
    cat.set_product("beer", r#"{"id":"beer","name":"Beer","categoryId":"drinks","price":300,"bom":[]}"#);
    cat.set_product("sake", r#"{"id":"sake","name":"Sake","bom":[{"supply":"salmon","qty":40}]}"#);
    cat
}

#[test]
fn a_dish_without_a_recipe_is_linked_to_its_own_piece() {
    let mut cat = catalog();
    let ids = vec!["cola".to_string(), "beer".to_string()];
    assert_eq!(without_recipe(&cat).len(), 2, "cola and the empty-recipe beer; sake reduces stock already");
    let r = link(&mut cat, &ids);
    assert_eq!(r["linked"].as_array().unwrap().len(), 2);
    let s: Value = serde_json::from_str(&cat.supply("asis-cola").unwrap()).unwrap();
    assert_eq!((s["name"].clone(), s["unit"].clone(), s["kind"].clone(), s["category"].clone()), (json!("Cola 0.33"), json!("unit"), json!("resale"), json!("Drinks")));
    // The ledger reads the new recipe: one cola sold reserves one piece.
    let p = cat.product("cola").unwrap();
    let evs = dowiz_hub::stock::reservations_for("o1", &[(p, 3)]);
    assert_eq!(evs, vec![dowiz_hub::stock::StockEvent::Reserved { item: "asis-cola".into(), qty: 3, order_id: "o1".into() }]);
    assert!(without_recipe(&cat).is_empty(), "nothing left that does not reduce stock");
}

/// Twins: a dish WITH a recipe is left alone; an unknown id is named; linking
/// twice reuses the same item and keeps what the owner set on it since.
#[test]
fn a_recipe_is_kept_and_a_second_tap_changes_nothing() {
    let mut cat = catalog();
    let r = link(&mut cat, &["sake".into(), "ghost".into()]);
    assert_eq!(r["linked"], json!([]));
    assert_eq!(r["skipped"], json!([{ "id": "sake", "why": "has a recipe" }, { "id": "ghost", "why": "no such dish" }]));
    assert!(cat.product("sake").unwrap().contains("salmon"));
    link(&mut cat, &["cola".into()]);
    cat.set_supply("asis-cola", r#"{"id":"asis-cola","name":"Cola can","unit":"unit","kind":"resale","lowAt":24}"#);
    let again = link(&mut cat, &["cola".into()]);
    assert_eq!(again["skipped"][0]["why"], json!("has a recipe"), "it is linked already");
    assert!(cat.supply("asis-cola").unwrap().contains("\"lowAt\":24"));
    assert_eq!(item_of(&"x".repeat(80)).len(), 64);
}
