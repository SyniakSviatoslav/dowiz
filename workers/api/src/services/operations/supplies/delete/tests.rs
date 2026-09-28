//! W-NOM: an ingredient deleted from the catalogue leaves every recipe, and
//! each dish that used it is re-derived from what is left.
use super::*;

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
    let lines = |v: &[(&str, i64)]| v.iter().map(|(s, q)| BomLineIn { supply: s.to_string(), qty: *q, net: None, out: None }).collect::<Vec<_>>();
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
