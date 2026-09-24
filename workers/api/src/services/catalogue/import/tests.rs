//! The imported record: what a re-import keeps. The row is parsed by the REAL
//! hub parser and priced by the REAL `price_basket`, so the test follows the
//! station from the owner's file to the line a bell is rung from.

use super::*;
use crate::bell_route::Station;
use crate::services::ordering::pricing::{price_basket, Want};

const CSV: &str = "Category,Name,Description,Price,Available\nDrinks,Beer,cold,300,yes\n";

fn draft() -> dowiz_hub::import::DraftProduct {
    let d = dowiz_hub::import::from_csv(CSV);
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
    d.products[0].clone()
}

/// The station on a re-imported dish, as the order line would carry it.
fn station_after_import(old: Option<&Value>) -> (Value, Station) {
    let p = draft();
    let rec = imported_product(&p, old);
    let json = rec.to_string();
    let basket = price_basket(|id| (id == p.id).then(|| json.clone()), [Want { product_id: &p.id, modifier_ids: &[], quantity: 1 }])
        .map_err(|r| r.text())
        .expect("the imported dish is sellable");
    (rec["station"].clone(), basket.lines[0].station)
}

#[test]
fn a_reimported_bar_dish_stays_at_the_bar() {
    let old = json!({"id": "drinks-beer", "name": "Beer", "price": 250, "station": "bar", "allergens": ["gluten"]});
    let (kept, line) = station_after_import(Some(&old));
    assert_eq!(kept, json!("bar"), "the file has no station column; the dish keeps its own");
    assert_eq!(line, Station::Bar, "and the order line is rung at the bar");
    // The rest of what the file has no column for is kept beside it, and the
    // file's own columns win.
    let rec = imported_product(&draft(), Some(&old));
    assert_eq!((rec["allergens"].clone(), rec["price"].clone()), (json!(["gluten"]), json!(300)));
}

/// TWIN: a dish the file brings for the first time has no station, which is
/// the kitchen, and a kitchen dish stays the kitchen's.
#[test]
fn a_new_or_kitchen_dish_is_the_kitchens() {
    assert_eq!(station_after_import(None), (Value::Null, Station::Kitchen));
    let old = json!({"id": "drinks-beer", "station": "kitchen"});
    assert_eq!(station_after_import(Some(&old)), (json!("kitchen"), Station::Kitchen));
}

/// F1: a re-imported price list must not erase the dish's RECIPE. It used to:
/// `imported_product` kept eight named keys and `bom` was not one of them, so
/// every recipe -- and every reservation the stock ledger makes from it --
/// vanished the day the owner re-uploaded a menu to change a price.
#[test]
fn a_reimported_dish_keeps_its_recipe_and_what_follows_from_it() {
    let old = json!({"id": "drinks-beer", "name": "Beer", "price": 250,
        "bom": [{"supply": "beer-keg", "qty": 500}], "weightG": 500, "cost": 120,
        "nutrition": {"kcal": 215}, "ingredients": ["beer"], "taste": {"sweet": 1},
        "unavailableNote": "keg empty", "available": false});
    let rec = imported_product(&draft(), Some(&old));
    assert_eq!(rec["bom"], old["bom"]);
    let reserved = dowiz_hub::stock::bom_of(&rec.to_string());
    assert_eq!(reserved.len(), 1, "the ledger still reserves the keg");
    for k in ["weightG", "cost", "nutrition", "ingredients", "taste"] {
        assert_eq!(rec[k], old[k], "{k}");
    }
    // The file's own columns still win, and "on sale" clears the old reason.
    assert_eq!((rec["price"].clone(), rec["available"].clone()), (json!(300), json!(true)));
    assert_eq!(rec["unavailableNote"], Value::Null);
    // TWIN: a new dish has no recipe to keep.
    assert_eq!(imported_product(&draft(), None)["bom"], Value::Null);
}
