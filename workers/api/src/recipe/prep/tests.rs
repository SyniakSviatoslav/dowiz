//! A ПФ line derives its cost exactly and its nutrition from its card, and a
//! recipe with no ПФ derives byte for byte what it did.

use super::*;
use crate::recipe::{derive, lines_of_stored};

fn book(id: &str) -> Option<String> {
    Some(match id {
        "rice-dry" => json!({ "id": "rice-dry", "name": "Rice", "unit": "g", "kind": "food_ingredient", "costPerBasis": 20, "kcalPer100": 350, "proteinPer100": 7 }),
        "vinegar" => json!({ "id": "vinegar", "name": "Vinegar", "unit": "ml", "kind": "condiment", "costPerBasis": 15, "kcalPer100": 20, "proteinPer100": 0 }),
        "salt" => json!({ "id": "salt", "name": "Salt", "unit": "g", "kind": "condiment", "costPerBasis": 5, "kcalPer100": 0, "proteinPer100": 0 }),
        "sugar" => json!({ "id": "sugar", "name": "Sugar", "unit": "g", "kind": "food_ingredient", "costPerBasis": 12, "kcalPer100": 400, "proteinPer100": 0 }),
        "water" => json!({ "id": "water", "name": "Water", "unit": "ml", "kind": "food_ingredient", "untracked": true, "kcalPer100": 0, "proteinPer100": 0 }),
        "mitsukan" => json!({ "id": "mitsukan", "name": "Mitsukan", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "vinegar", "qty": 800 }, { "item": "salt", "qty": 50 }, { "item": "sugar", "qty": 150 }], "yield": 1000 } }),
        "rice-seasoned" => json!({ "id": "rice-seasoned", "name": "Rice seasoned", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "rice-dry", "qty": 1000 }, { "item": "water", "qty": 1100 }, { "item": "mitsukan", "qty": 250 }], "yield": 2100 } }),
        "egg" => json!({ "id": "egg", "name": "Egg", "unit": "unit", "kind": "food_ingredient", "costPerBasis": 25, "kcalPer100": 70 }),
        "mayo" => json!({ "id": "mayo", "name": "Mayo", "unit": "ml", "kind": "prep", "card": { "lines": [{ "item": "egg", "qty": 2 }, { "item": "vinegar", "qty": 10 }], "yield": 250 } }),
        _ => return None,
    }
    .to_string())
}

#[test]
fn a_prep_is_hydrated_with_exact_cost_and_nutrition_per_basis() {
    let v: Value = serde_json::from_str(&hydrated("mitsukan", &book("mitsukan").unwrap(), &book)).unwrap();
    assert_eq!(v["costMicroPerUnit"], json!(140_500), "0.1405 lek per g");
    assert_eq!(v["costPerBasis"], json!(14), "14.05 per 100 g, for the console");
    // 800 ml vinegar 160 kcal + 150 g sugar 600 kcal = 760 kcal per 1000 g -> 76 per 100.
    assert_eq!((v["kcalPer100"].clone(), v["proteinPer100"].clone(), v["nutritionBasis"].clone()), (json!(76.0), json!(0.0), json!("cooked")));
    assert_eq!(v["k"], json!(1000));
    let r: Value = serde_json::from_str(&hydrated("rice-seasoned", &book("rice-seasoned").unwrap(), &book)).unwrap();
    assert_eq!(r["costMicroPerUnit"], json!(111_964));
    // 1000 g rice 3500 kcal + 250 g mitsukan 190 kcal = 3690 / 2100 * 100 = 175.7
    assert_eq!(r["kcalPer100"], json!(175.7));
    assert_eq!(r["k"], json!(894));
    // A raw supply is handed back untouched.
    assert_eq!(hydrated("salt", &book("salt").unwrap(), &book), book("salt").unwrap());
}

#[test]
fn a_dish_line_naming_a_prep_costs_and_weighs_exactly() {
    let lines = lines_of_stored(&json!([{ "supply": "rice-seasoned", "qty": 130 }]), book);
    let l = &lines[0];
    assert_eq!((l.name.as_str(), l.kind.as_str(), l.cost), ("Rice seasoned", "prep", Some(15)), "130 x 0.111964 = 14.56");
    assert_eq!((l.w.gross, l.w.net, l.w.out, l.weight_g), (Some(130), Some(130), Some(130), Some(130.0)), "no losses on a yield");
    assert_eq!(l.kcal.map(|k| k.round()), Some(228.0), "175.7 per 100 g cooked x 130");
    let d = derive(&lines);
    assert_eq!((d.cost, d.weight_g, d.kcal), (Some(15), Some(130), 228));
    assert!(d.nutrition_complete);
    assert_eq!(d.ingredients, vec!["Rice seasoned".to_string()]);
}

#[test]
fn pieces_inside_a_prep_count_and_cost_by_the_piece_and_k_is_unknown() {
    let v: Value = serde_json::from_str(&hydrated("mayo", &book("mayo").unwrap(), &book)).unwrap();
    // 2 eggs at 25 + 10 ml vinegar at 0.15 = 51.5 lek per 250 ml = 0.206 per ml.
    assert_eq!(v["costMicroPerUnit"], json!(206_000));
    assert_eq!(v["k"], Value::Null, "an egg without a weight: K unknown, not refused");
    assert_eq!(v["kcalPer100"], json!(56.8), "(140 + 2) / 250 * 100");
    assert_eq!(price_micro(&book("egg").unwrap()), Some(25_000_000), "per piece");
    assert_eq!(price_micro(&book("salt").unwrap()), Some(50_000), "5 per 100 g");
    assert_eq!(price_micro(&book("mayo").unwrap()), None, "a prep has no list price of its own");
}

#[test]
fn the_view_names_each_line_and_prices_the_batch() {
    let v = view("rice-seasoned", &book("rice-seasoned").unwrap(), &book);
    assert_eq!(v["yield"], json!(2100));
    assert_eq!(v["costPer"], json!(112), "per kg");
    assert_eq!(v["batchCost"], json!(235));
    assert_eq!(v["grossG"], json!(2350));
    let names: Vec<&str> = v["lines"].as_array().unwrap().iter().map(|l| l["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Rice", "Water", "Mitsukan"]);
    assert_eq!(v["lines"][1]["untracked"], json!(true));
    assert_eq!(v["lines"][2]["cost"], json!(35), "250 g of mitsukan at 0.1405");
    let missing = view("m", r#"{"id":"m","kind":"prep","unit":"g","card":{"lines":[{"item":"gone","qty":1}],"yield":1}}"#, &book);
    assert_eq!(missing["lines"][0]["missing"], json!(true));
    assert_eq!(missing["costPer"], Value::Null);
}
