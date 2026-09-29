//! R1 over a REAL catalogue image: the recipes file's semi-finished cards
//! become ПФ supplies through the console's own card writer, the dishes name
//! them, a second import changes nothing, and a flat file writes exactly what
//! it wrote before this row.

use super::super::*;
use dowiz_hub::import::recipes::recipes_against;

const NOW: i64 = 1_790_078_400_000;

const SUPPLIES: &str = "name,unit,cost,per\nSalmon,g,2400,1 kg\nRice,g,200,1 kg\nMayo,g,600,1 kg\nSriracha,g,900,1 kg\n";

const SAUCE: &str = "dish,ingredient,qty,unit,batch\n\
    Spicy mayo,Mayo,800,g,1 kg\n\
    Spicy mayo,Sriracha,200,g,\n\
    Maki Salmon,Salmon,40,g,\n\
    Maki Salmon,Spicy mayo,15,g,\n\
    Maki Salmon,Rice,100,g,\n";

const FLAT: &str = "dish,ingredient,qty,unit\nMaki Salmon,Salmon,40,g\nMaki Salmon,Rice,100,g\n";

fn venue() -> Catalog {
    let mut cat = Catalog::create().expect("an empty catalogue");
    cat.set_location(&json!({ "id": "v1", "currency_code": "ALL", "menu_version": 4 }).to_string());
    cat.set_product("maki-salmon", &json!({ "id": "maki-salmon", "name": "Maki Salmon", "price": 650 }).to_string());
    let (d, _) = read(&cat, SUPPLIES, Kind::Supplies, CostScale::Major, NOW);
    apply_supplies(&mut cat, &d, false).expect("supplies land");
    cat
}

fn json_of(j: Option<String>) -> Value {
    serde_json::from_str(&j.expect("a record")).expect("json")
}

#[test]
fn a_semi_finished_card_lands_as_a_prep_the_dish_names() {
    let mut cat = venue();
    let (d, rows) = read(&cat, SAUCE, Kind::Recipes, CostScale::Major, NOW);
    assert_eq!(d.preps.len(), 1, "{:?}", d.warnings);
    // The dry run judges the dish against the ПФ it will have: no "unknown supply".
    assert_eq!(rows[0]["error"], Value::Null, "{rows:?}");
    assert_eq!(rows[0]["after"]["lines"], json!(3));
    // The preview names what it creates, with its lines by name and its K.
    let shown = preps::rows(&cat, &d);
    assert_eq!((shown[0]["id"].clone(), shown[0]["new"].clone(), shown[0]["yield"].clone()), (json!("spicy-mayo"), json!(true), json!(1000)));
    assert_eq!(shown[0]["lines"][0]["name"], json!("Mayo"));
    assert_eq!(shown[0]["k"], json!(1000), "1000 g out of 1000 g in");
    assert!(cat.supply("spicy-mayo").is_none(), "a dry run writes nothing");

    assert_eq!(apply_recipes(&mut cat, &d), Ok(2), "one ПФ and one dish");
    let sauce = json_of(cat.supply("spicy-mayo"));
    assert_eq!((sauce["kind"].clone(), sauce["card"].clone()), (json!("prep"), json!({ "lines": [{ "item": "mayo", "qty": 800 }, { "item": "sriracha", "qty": 200 }], "yield": 1000 })));
    let dish = json_of(cat.product("maki-salmon"));
    let names: Vec<(String, i64)> = dowiz_hub::stock::bom_of(&dish.to_string()).into_iter().map(|l| (l.supply, l.qty)).collect();
    assert_eq!(names, vec![("salmon".into(), 40), ("spicy-mayo".into(), 15), ("rice".into(), 100)]);
    // A sale still takes raw leaves: 15 g of the sauce is 12 g mayo, 3 g sriracha.
    let takes = crate::services::operations::preps::takes(&cat, &dish.to_string());
    let leaves: Vec<(String, i64)> = takes["leaves"].as_array().unwrap().iter().map(|l| (l["supply"].as_str().unwrap().to_string(), l["uq"].as_i64().unwrap())).collect();
    assert_eq!(leaves, vec![("mayo".into(), 12_000_000), ("rice".into(), 100_000_000), ("salmon".into(), 40_000_000), ("sriracha".into(), 3_000_000)]);
}

#[test]
fn the_same_file_twice_changes_nothing() {
    let mut cat = venue();
    let (d, _) = read(&cat, SAUCE, Kind::Recipes, CostScale::Major, NOW);
    apply_recipes(&mut cat, &d).unwrap();
    // The menu version moves on every Apply (the storefront re-reads); the records must not.
    let once = (cat.supplies(), cat.products());
    let (again, _) = read(&cat, SAUCE, Kind::Recipes, CostScale::Major, NOW);
    assert!(again.warnings.is_empty(), "the ПФ it made is not a clash: {:?}", again.warnings);
    assert_eq!(preps::rows(&cat, &again)[0]["new"], json!(false), "the preview says update");
    apply_recipes(&mut cat, &again).unwrap();
    assert_eq!((cat.supplies(), cat.products()), once, "a second import is an update to the same bytes");
}

#[test]
fn a_flat_file_writes_what_it_wrote_before_this_row() {
    // The reading before R1 (`preps: false`) against the one after it.
    let before = {
        let mut cat = venue();
        let products = vec![("maki-salmon".to_string(), "Maki Salmon".to_string())];
        let (known, existing) = preps::supplies_of(&cat);
        let o = dowiz_hub::import::recipes::Opts { currency: "ALL", cost_scale: Some(CostScale::Major), unit_hint: None, local_now_ms: NOW, products: &products, existing_supplies: &existing, preps: false };
        apply_recipes(&mut cat, &recipes_against(FLAT, &o, &known)).unwrap();
        cat.root()
    };
    let mut cat = venue();
    let (d, _) = read(&cat, FLAT, Kind::Recipes, CostScale::Major, NOW);
    assert!(d.preps.is_empty());
    apply_recipes(&mut cat, &d).unwrap();
    assert_eq!(cat.root(), before);
}

#[test]
fn a_supplies_file_never_retires_a_semi_finished_product() {
    let mut cat = venue();
    let (d, _) = read(&cat, SAUCE, Kind::Recipes, CostScale::Major, NOW);
    apply_recipes(&mut cat, &d).unwrap();
    // An ingredients file lists raw items; the sauce is not in it and must stay.
    let (d, _) = read(&cat, "name,unit\nSalmon,g\n", Kind::Supplies, CostScale::Major, NOW);
    assert!(!d.retired.contains(&"spicy-mayo".to_string()), "{:?}", d.retired);
    assert_eq!(d.retired.len(), 3, "mayo, rice, sriracha: the raw ones: {:?}", d.retired);
    apply_supplies(&mut cat, &d, true).unwrap();
    assert_eq!(json_of(cat.supply("spicy-mayo"))["active"], json!(true));
    assert_eq!(json_of(cat.supply("rice"))["active"], json!(false), "the twin: a raw one is retired");
}
