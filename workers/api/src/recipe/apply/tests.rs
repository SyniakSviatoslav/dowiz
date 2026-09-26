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
    BomLineIn { supply: s.into(), qty, net: None, out: None }
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

// ── G11: recipe <-> published values (audit D19, D40) ───────────────────────

/// D19: the bulk import writes with `Typed::from_record`, so a kcal, a weight
/// and an ingredient list the owner typed survive a recipe re-import. With
/// `Typed::default()` -- what bulk.rs passed -- all three were overwritten.
#[test]
fn a_reimported_recipe_keeps_what_the_owner_typed() {
    let mut p = dish();
    let typed = Typed { nutrition: true, weight: true, ingredients: true };
    p["ingredients"] = json!(["salmon", "love"]);
    set_bom(&mut p, &[line("salmon", 35)], supply, typed).unwrap();
    let marks = Typed::from_record(&p);
    assert!(marks.nutrition && marks.weight && marks.ingredients, "the record remembers who typed what");
    let kept = Typed::from_record(&p);
    set_bom(&mut p, &[line("rice", 90), line("salmon", 35)], supply, kept).unwrap();
    assert_eq!((p["nutrition"]["kcal"].clone(), p["weightG"].clone()), (json!(1), json!(999)));
    assert_eq!(p["ingredients"], json!(["salmon", "love"]));
    assert_eq!(p["cost"], json!(34 + 84), "the cost still follows the new lines");
    // TWIN: a dish whose values came from the recipe re-derives them.
    let mut q = dish();
    set_bom(&mut q, &[line("salmon", 35)], supply, Typed::default()).unwrap();
    let kept = Typed::from_record(&q);
    set_bom(&mut q, &[line("rice", 90), line("salmon", 35)], supply, kept).unwrap();
    assert_eq!((q["nutrition"]["kcal"].clone(), q["weightG"].clone()), (json!(190), json!(125)));
    assert_eq!(q["ingredients"], json!(["Rice", "Salmon"]));
}

/// D40: clearing a recipe clears what it derived -- the storefront was left
/// publishing kcal and ingredients for a recipe that no longer existed -- and
/// keeps what the owner typed.
#[test]
fn clearing_a_recipe_clears_only_what_it_derived() {
    let mut p = dish();
    set_bom(&mut p, &[line("rice", 90), line("salmon", 35)], supply, Typed::default()).unwrap();
    set_bom(&mut p, &[], supply, Typed::default()).unwrap();
    for k in ["nutrition", "weightG", "ingredients", "bom", "cost", "nutritionComplete"] {
        assert_eq!(p[k], Value::Null, "{k} followed the recipe out");
    }
    // TWIN: typed values stay.
    let mut q = dish();
    q["ingredients"] = json!(["salmon"]);
    set_bom(&mut q, &[line("salmon", 35)], supply, Typed { nutrition: true, weight: true, ingredients: true }).unwrap();
    set_bom(&mut q, &[], supply, Typed::default()).unwrap();
    assert_eq!((q["nutrition"]["kcal"].clone(), q["weightG"].clone()), (json!(1), json!(999)));
    assert_eq!(q["ingredients"], json!(["salmon"]));
}

/// R6 THROUGH THE ONE WRITE: a weighed line is stored with its net and out,
/// the dish weighs its OUT, and a net heavier than its gross writes nothing.
#[test]
fn a_weighed_line_is_stored_and_the_dish_weighs_its_out() {
    let mut p = dish();
    let fillet = BomLineIn { supply: "salmon".into(), qty: 100, net: Some(55), out: Some(50) };
    set_bom(&mut p, &[fillet, line("rice", 90)], supply, Typed::default()).unwrap();
    assert_eq!(p["bom"], json!([{ "supply": "salmon", "qty": 100, "net": 55, "out": 50 }, { "supply": "rice", "qty": 90 }]));
    assert_eq!(p["weightG"], json!(140), "50 g of salmon on the plate + 90 g rice");
    // Nutrition follows the edible 55 of the 100 g gross: 208 x 0.55 = 114.4, + 117 rice.
    assert_eq!(p["nutrition"]["kcal"], json!(231));
    assert_eq!(p["cost"], json!(240 + 34), "cost is the GROSS the kitchen paid for");
    // Refusal twin: a net above the gross, and nothing changed on the dish.
    let before = p.clone();
    let bad = BomLineIn { supply: "salmon".into(), qty: 100, net: Some(101), out: None };
    let e = set_bom(&mut p, &[bad], supply, Typed::default()).unwrap_err();
    assert!(e.contains("salmon") && e.contains("more than the gross"), "{e}");
    assert_eq!(p, before);
}
