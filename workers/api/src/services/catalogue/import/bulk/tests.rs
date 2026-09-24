//! F1 over a REAL catalogue image: the file is read by the hub's parser and
//! written by the console's own writers, and the dish's numbers follow.

use super::*;

const NOW: i64 = 1_790_078_400_000;

const SUPPLIES: &str = "emri;lloji;njësia;kalori;proteina;yndyra;karbohidrate;kosto;për;minimumi\n\
    Oriz sushi;food;kg;130;2,7;0,3;28;380;1 kg;5 kg\n\
    Nori;food;g;35;5,8;0,3;5,1;4.500,00;1 kg;200 g\n\
    Salmon;food;kg;208;20;13;0;2.400,00;;3 kg\n\
    Kuti;box;unit;;;;;35;;50\n\
    Sake;food;cup;;;;;;;\n";

const RECIPES: &str = "dish,ingredient,qty,unit\n\
    Maki Salmon,Oriz sushi,90,g\nMaki Salmon,Nori,4,g\nMaki Salmon,Salmon,35,g\nMaki Salmon,Kuti,1,unit\n\
    Maki Tuna,Oriz sushi,90,g\n";

fn venue() -> Catalog {
    let mut cat = Catalog::create().expect("an empty catalogue");
    cat.set_location(&json!({ "id": "v1", "currency_code": "ALL", "menu_version": 4 }).to_string());
    cat.set_product("maki-salmon", &json!({ "id": "maki-salmon", "name": "Maki Salmon", "price": 650, "weightG": 200, "nutrition": { "kcal": 999 } }).to_string());
    cat.set_product("maki-cucumber", &json!({ "id": "maki-cucumber", "name": "Maki Cucumber", "price": 550 }).to_string());
    cat
}

fn product(cat: &Catalog, id: &str) -> Value {
    serde_json::from_str(&cat.product(id).unwrap()).unwrap()
}

fn supplied() -> Catalog {
    let mut cat = venue();
    let (d, _) = read(&cat, SUPPLIES, Kind::Supplies, CostScale::Major, NOW);
    assert_eq!(apply_supplies(&mut cat, &d, false), Ok(4), "{:?}", d.warnings);
    cat
}

#[test]
fn a_supplies_file_lands_through_the_forms_writer() {
    let cat = supplied();
    let salmon: Value = serde_json::from_str(&cat.supply("salmon").unwrap()).unwrap();
    // "2.400,00" per kg is 240 lek per 100 g; 3 kg of threshold is 3000 g.
    assert_eq!((salmon["unit"].clone(), salmon["costPerBasis"].clone(), salmon["lowAt"].clone()), (json!("g"), json!(240), json!(3000)));
    assert_eq!((salmon["active"].clone(), salmon["nutritionConfirmed"].clone()), (json!(true), json!(false)));
    let kuti: Value = serde_json::from_str(&cat.supply("kuti").unwrap()).unwrap();
    assert_eq!((kuti["kind"].clone(), kuti["unit"].clone(), kuti["costPerBasis"].clone()), (json!("packaging"), json!("unit"), json!(35)));
    // The cup is a warning naming its row, and nothing was written for it.
    assert!(cat.supply("sake").is_none());
    let (d, rows) = read(&venue(), SUPPLIES, Kind::Supplies, CostScale::Major, NOW);
    assert!(d.warnings.iter().any(|w| w.contains("row 6") && w.contains("cup")), "{:?}", d.warnings);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0]["new"], json!(true));
}

#[test]
fn a_vanished_supply_is_retired_only_when_asked() {
    let mut cat = supplied();
    let (d, _) = read(&cat, "name,unit\nSalmon,kg\n", Kind::Supplies, CostScale::Major, NOW);
    assert_eq!(d.retired.len(), 3, "{:?}", d.retired);
    apply_supplies(&mut cat, &d, false).unwrap();
    let nori: Value = serde_json::from_str(&cat.supply("nori").unwrap()).unwrap();
    assert_eq!(nori["active"], json!(true), "not asked: kept");
    apply_supplies(&mut cat, &d, true).unwrap();
    let nori: Value = serde_json::from_str(&cat.supply("nori").unwrap()).unwrap();
    assert_eq!(nori["active"], json!(false), "asked: retired, never deleted");
}

#[test]
fn a_recipes_file_lands_as_bom_and_the_dish_numbers_follow() {
    let mut cat = supplied();
    let (d, rows) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    assert!(d.warnings.iter().any(|w| w.contains("Maki Tuna") && w.contains("not on the menu")), "{:?}", d.warnings);
    // The dry run says what would change, and changes nothing.
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0]["before"]["lines"].clone(), rows[0]["after"]["lines"].clone()), (json!(0), json!(4)));
    assert_eq!(product(&cat, "maki-salmon")["bom"], Value::Null, "a dry run writes nothing");

    assert_eq!(apply_recipes(&mut cat, &d), Ok(1));
    let p = product(&cat, "maki-salmon");
    let reserved: Vec<(String, i64)> = dowiz_hub::stock::bom_of(&p.to_string()).into_iter().map(|l| (l.supply, l.qty)).collect();
    assert_eq!(reserved, vec![("oriz-sushi".into(), 90), ("nori".into(), 4), ("salmon".into(), 35), ("kuti".into(), 1)]);
    // 90 g rice 117 + 4 g nori 1.4 + 35 g salmon 72.8 = 191.2 kcal.
    assert_eq!(p["nutrition"]["kcal"], json!(191));
    assert_eq!(p["weightG"], json!(129), "the food lines' grams");
    // Per line, rounded: rice 34.2 -> 34, nori 18, salmon 84, the box 35.
    assert_eq!(p["cost"], json!(171));
    assert_eq!(p["nutritionComplete"], json!(true));
    assert_eq!(rows[0]["after"]["kcal"], p["nutrition"]["kcal"], "the preview is the write");
    // The dish nobody named is untouched; the menu version moved once.
    assert_eq!(product(&cat, "maki-cucumber")["bom"], Value::Null);
    let loc: Value = serde_json::from_str(&cat.location().unwrap()).unwrap();
    assert_eq!(loc["menu_version"], json!(5));
}

#[test]
fn the_same_file_twice_lands_the_same_recipe() {
    let mut cat = supplied();
    let (d, _) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    apply_recipes(&mut cat, &d).unwrap();
    let once = product(&cat, "maki-salmon");
    apply_recipes(&mut cat, &d).unwrap();
    assert_eq!(product(&cat, "maki-salmon"), once);
}

#[test]
fn recipes_before_supplies_import_nothing_and_say_so() {
    let mut cat = venue();
    let (d, rows) = read(&cat, RECIPES, Kind::Recipes, CostScale::Major, NOW);
    assert!(rows.is_empty() && d.recipes.is_empty());
    assert!(d.warnings.iter().any(|w| w.contains("supplies file first")), "{:?}", d.warnings);
    assert_eq!(apply_recipes(&mut cat, &d), Ok(0));
}
