//! The one recipe write, as both the dish sheet and the bulk import call it.

use super::*;

fn supply(id: &str) -> Option<String> {
    match id {
        "salmon" => Some(json!({ "name": "Salmon", "unit": "g", "kind": "food_ingredient", "kcalPer100": 208, "proteinPer100": 20, "fatPer100": 13, "carbsPer100": 0, "costPerBasis": 240 }).to_string()),
        "rice" => Some(json!({ "name": "Rice", "unit": "g", "kind": "food_ingredient", "kcalPer100": 130, "proteinPer100": 2.7, "fatPer100": 0.3, "carbsPer100": 28, "costPerBasis": 38 }).to_string()),
        "box" => Some(json!({ "name": "Box", "unit": "unit", "kind": "packaging", "costPerBasis": 35 }).to_string()),
        _ => None,
    }
}

fn line(s: &str, qty: i64) -> BomLineIn {
    BomLineIn { supply: s.into(), qty }
}

fn dish() -> Value {
    json!({ "id": "maki-salmon", "name": "Maki Salmon", "price": 650, "weightG": 999, "nutrition": { "kcal": 1 } })
}

#[test]
fn an_untyped_recipe_derives_kcal_weight_and_cost() {
    let mut p = dish();
    let d = set_bom(&mut p, &[line("rice", 90), line("salmon", 35), line("box", 1)], supply, Typed::default()).unwrap();
    assert!(d.is_some());
    // 90 g rice = 117 kcal, 35 g salmon = 72.8 kcal.
    assert_eq!(p["nutrition"]["kcal"], json!(190));
    assert_eq!(p["nutritionDerived"], json!(true));
    assert_eq!(p["weightG"], json!(125), "the food lines' grams replace a hand-typed weight");
    // 90 × 38 / 100 = 34.2 → 34; 35 × 240 / 100 = 84; the box 35.
    assert_eq!(p["cost"], json!(34 + 84 + 35));
    assert_eq!(p["ingredients"], json!(["Rice", "Salmon"]));
    // The ledger reads the two keys it needs from what was stored.
    let read = dowiz_hub::stock::bom_of(&p.to_string());
    let got: Vec<(String, i64)> = read.into_iter().map(|l| (l.supply, l.qty)).collect();
    assert_eq!(got, vec![("rice".into(), 90), ("salmon".into(), 35), ("box".into(), 1)]);
}

#[test]
fn what_the_owner_typed_wins_over_the_sum() {
    let mut p = dish();
    let typed = Typed { nutrition: true, weight: true, ingredients: true };
    set_bom(&mut p, &[line("salmon", 35)], supply, typed).unwrap();
    assert_eq!((p["nutrition"]["kcal"].clone(), p["weightG"].clone()), (json!(1), json!(999)));
    assert_eq!(p["nutritionDerived"], json!(false));
    assert_eq!(p["cost"], json!(84), "cost is never typed; it always follows the lines");
}

#[test]
fn an_unknown_supply_refuses_the_whole_write() {
    let mut p = dish();
    let before = p.clone();
    let e = set_bom(&mut p, &[line("salmon", 35), line("wasabi", 5)], supply, Typed::default()).unwrap_err();
    assert_eq!(e, "unknown supply wasabi");
    assert_eq!(p, before, "nothing half-written");
    // TWIN: the same lines without it land.
    assert!(set_bom(&mut p, &[line("salmon", 35)], supply, Typed::default()).unwrap().is_some());
}

#[test]
fn no_lines_clears_the_recipe_and_a_repeat_is_dropped() {
    let mut p = dish();
    set_bom(&mut p, &[line("salmon", 35), line("salmon", 50)], supply, Typed::default()).unwrap();
    assert_eq!(p["bom"].as_array().map(Vec::len), Some(1));
    assert_eq!(p["bom"][0]["qty"], json!(35));
    assert!(set_bom(&mut p, &[], supply, Typed::default()).unwrap().is_none());
    assert_eq!((p["bom"].clone(), p["cost"].clone()), (Value::Null, Value::Null));
}
