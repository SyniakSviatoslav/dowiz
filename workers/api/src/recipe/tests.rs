//! Scaling a line, summing a dish, and reading what was stored.

use super::*;

fn salmon() -> Value {
    json!({ "name": "Salmon", "unit": "g", "kind": "food_ingredient", "kcalPer100": 208, "proteinPer100": 20.4, "fatPer100": 13.4, "carbsPer100": 0, "costPerBasis": 1800 })
}
fn box_() -> Value {
    json!({ "name": "Box", "unit": "unit", "kind": "packaging", "costPerBasis": 40, "weightPerUnit": 12 })
}

#[test]
fn a_line_scales_per_hundred_and_per_piece() {
    let s = line_of("salmon", 40, &salmon());
    assert_eq!(s.kcal.map(|x| x.round()), Some(83.0));
    assert_eq!(s.cost, Some(720));
    assert_eq!(s.weight_g, Some(40.0));
    let b = line_of("box", 2, &box_());
    assert_eq!(b.kcal, None);
    assert_eq!(b.cost, Some(80));
    assert_eq!(b.weight_g, Some(24.0));
}

#[test]
fn the_dish_sums_food_only_and_costs_everything() {
    let lines = vec![line_of("salmon", 40, &salmon()), line_of("box", 1, &box_())];
    let d = derive(&lines);
    assert_eq!(d.kcal, 83);
    assert_eq!(d.protein, 8);
    assert!(d.nutrition_complete);
    assert_eq!(d.cost, Some(760));
    assert_eq!(d.weight_g, Some(40));
    assert_eq!(d.ingredients, vec!["Salmon".to_string()]);
}

#[test]
fn a_food_line_without_kcal_makes_the_sum_incomplete() {
    let rice = json!({ "name": "Rice", "unit": "g", "kind": "food_ingredient" });
    let d = derive(&[line_of("salmon", 40, &salmon()), line_of("rice", 90, &rice)]);
    assert!(!d.nutrition_complete);
    assert_eq!(d.cost, None);
    assert_eq!(d.weight_g, Some(130));
}

#[test]
fn bom_json_keeps_the_ledgers_keys_first() {
    let v = bom_json(&[line_of("salmon", 40, &salmon())]);
    let lines = dowiz_hub::stock::bom_of(&json!({ "bom": v }).to_string());
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].supply, "salmon");
    assert_eq!(lines[0].qty, 40);
}

#[test]
fn taste_is_five_axes_at_three_levels() {
    let ok = validate_taste(&serde_json::from_value(json!({ "spicy": 3, "sweet": 0 })).unwrap()).unwrap();
    assert_eq!(ok.len(), 1);
    assert!(validate_taste(&serde_json::from_value(json!({ "umami": 2 })).unwrap()).is_err());
    assert!(validate_taste(&serde_json::from_value(json!({ "spicy": 5 })).unwrap()).is_err());
}

// ── what is stored, and how it is read back (2026-09-24) ────────────────────

fn book(id: &str) -> Option<String> {
    match id {
        "salmon" => Some(salmon().to_string()),
        "box" => Some(box_().to_string()),
        _ => None,
    }
}

#[test]
fn a_stored_line_is_its_supply_and_qty_and_nothing_else() {
    let v = bom_json(&[line_of("salmon", 40, &salmon()), line_of("box", 1, &box_())]);
    assert_eq!(v, json!([{ "supply": "salmon", "qty": 40 }, { "supply": "box", "qty": 1 }]));
    assert_eq!(bom_json(&[]), json!([]));
}

#[test]
fn a_lean_line_reads_back_from_the_supply_as_it_is_now() {
    let lines = lines_of_stored(&json!([{ "supply": "salmon", "qty": 40 }, { "supply": "box", "qty": 2 }]), book);
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].name.as_str(), lines[0].cost, lines[0].weight_g), ("Salmon", Some(720), Some(40.0)));
    assert_eq!((lines[1].kind.as_str(), lines[1].cost), ("packaging", Some(80)));
    let d = derive(&lines);
    assert_eq!((d.kcal, d.cost, d.weight_g), (83, Some(800), Some(40)));
}

/// A snapshot line stored before the change is live on a venue until the dish
/// is saved again: its supply's numbers NOW win over the snapshot, as the
/// console always drew them, and only a supply that is gone keeps it.
#[test]
fn a_snapshot_line_stored_before_still_reads() {
    let old = json!([
        { "supply": "salmon", "qty": 40, "name": "Salmon (old)", "unit": "g", "kind": "food_ingredient", "kcal": 1.0, "cost": 1, "weightG": 40.0 },
        { "supply": "tuna", "qty": 30, "name": "Tuna", "unit": "g", "kind": "food_ingredient", "kcal": 43.0, "protein": 7.0, "fat": 1.5, "carbs": 0.0, "cost": 780, "weightG": 30.0 },
    ]);
    let lines = lines_of_stored(&old, book);
    assert_eq!((lines[0].name.as_str(), lines[0].cost), ("Salmon", Some(720)), "the supply as it is now");
    let t = &lines[1];
    assert_eq!((t.name.as_str(), t.unit.as_str(), t.kcal, t.protein, t.cost, t.weight_g), ("Tuna", "g", Some(43.0), Some(7.0), Some(780), Some(30.0)));
    // The stored snapshot still reserves exactly as before.
    let read = dowiz_hub::stock::bom_of(&json!({ "bom": old }).to_string());
    assert_eq!(read.iter().map(|l| (l.supply.as_str(), l.qty)).collect::<Vec<_>>(), vec![("salmon", 40), ("tuna", 30)]);
}

#[test]
fn a_lean_line_whose_supply_is_gone_reads_as_its_id() {
    let lines = lines_of_stored(&json!([{ "supply": "yuzu", "qty": 5 }]), book);
    assert_eq!((lines[0].name.as_str(), lines[0].unit.as_str(), lines[0].kind.as_str()), ("yuzu", "g", "food_ingredient"));
    assert_eq!((lines[0].kcal, lines[0].cost, lines[0].weight_g), (None, None, None));
    assert!(!derive(&lines).nutrition_complete);
}

#[test]
fn what_is_not_a_line_is_not_read() {
    let junk = json!([{ "qty": 5 }, { "supply": "salmon" }, { "supply": "salmon", "qty": "40" }, { "supply": 7, "qty": 1 }]);
    assert!(lines_of_stored(&junk, book).is_empty());
    assert!(lines_of_stored(&json!("not an array"), book).is_empty());
    assert!(lines_of_stored(&Value::Null, book).is_empty());
}

#[test]
fn the_owner_reads_a_lean_bom_with_names_and_numbers() {
    let mut p = json!({ "id": "maki", "bom": [{ "supply": "salmon", "qty": 40 }] });
    hydrate(&mut p, book);
    assert_eq!(p["bom"][0]["name"], json!("Salmon"));
    assert_eq!((p["bom"][0]["kcal"].clone(), p["bom"][0]["cost"].clone(), p["bom"][0]["weightG"].clone()), (json!(83.0), json!(720), json!(40.0)));
    assert_eq!((p["bom"][0]["protein"].clone(), p["bom"][0]["unit"].clone()), (json!(8.2), json!("g")));
    // TWINS: a dish without a recipe, and a cleared one, are left as they are.
    for mut q in [json!({ "id": "water" }), json!({ "id": "w", "bom": null })] {
        let before = q.clone();
        hydrate(&mut q, book);
        assert_eq!(q, before);
    }
}

// ── gross, net, out (research 2026-09-26 R6) ───────────────────────────────

fn fish() -> Value {
    json!({ "name": "Salmon", "unit": "g", "kind": "food_ingredient", "kcalPer100": 200, "costPerBasis": 1800, "cleanPm": 550, "cookPm": 900 })
}

/// The supply's defaults: 100 g gross -> 55 net -> 50 out. The dish weighs
/// the OUT, nutrition follows the NET (raw basis), cost stays on the GROSS.
#[test]
fn a_line_follows_its_supplys_losses() {
    let l = line_of("fish", 100, &fish());
    assert_eq!((l.w.gross, l.w.net, l.w.out), (Some(100), Some(55), Some(50)));
    assert_eq!(l.weight_g, Some(50.0));
    assert_eq!(l.kcal.map(|k| k.round()), Some(110.0), "200 kcal/100 g x 55 g edible");
    assert_eq!(l.cost, Some(1800));
    assert_eq!(derive(&[l]).weight_g, Some(50));
}

/// Figures declared on the COOKED weight scale by the out instead.
#[test]
fn a_cooked_basis_scales_by_the_out() {
    let mut rice = json!({ "name": "Rice", "unit": "g", "kind": "food_ingredient", "kcalPer100": 130, "cookPm": 2200 });
    assert_eq!(line_of("rice", 100, &rice).kcal.map(|k| k.round()), Some(130.0), "raw basis: 100 g dry");
    rice["nutritionBasis"] = json!("cooked");
    assert_eq!(line_of("rice", 100, &rice).kcal.map(|k| k.round()), Some(286.0), "cooked basis: 220 g cooked");
}

/// A stored line with the owner's weighed numbers reads them back, writes
/// them back, and an unweighed line stays lean -- so an old recipe is
/// byte-identical after a save that changed nothing.
#[test]
fn weighed_numbers_round_trip_and_lean_lines_stay_lean() {
    let stored = json!([{ "supply": "fish", "qty": 100, "net": 60, "out": 45 }, { "supply": "box", "qty": 1 }]);
    let book2 = |id: &str| match id { "fish" => Some(fish().to_string()), other => book(other) };
    let lines = lines_of_stored(&stored, book2);
    assert_eq!((lines[0].w.net, lines[0].w.out, lines[0].w.net_set), (Some(60), Some(45), Some(60)));
    assert_eq!(bom_json(&lines), stored);
    let lean = json!([{ "supply": "salmon", "qty": 40 }]);
    assert_eq!(bom_json(&lines_of_stored(&lean, book)), lean);
}

/// The owner's view carries every weight and the loss.
#[test]
fn the_owner_sees_gross_net_out_and_the_loss() {
    let mut p = json!({ "bom": [{ "supply": "fish", "qty": 100 }] });
    hydrate(&mut p, |id| (id == "fish").then(|| fish().to_string()));
    let l = &p["bom"][0];
    assert_eq!((l["grossG"].clone(), l["netG"].clone(), l["outG"].clone()), (json!(100), json!(55), json!(50)));
    assert_eq!((l["cleanPm"].clone(), l["cookPm"].clone(), l["lossPm"].clone()), (json!(550), json!(900), json!(500)));
    assert_eq!((l["net"].clone(), l["out"].clone()), (Value::Null, Value::Null), "nothing typed by the owner");
}
