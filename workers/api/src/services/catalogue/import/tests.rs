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
    let rec = imported_product(&p, old, true);
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
    let rec = imported_product(&draft(), Some(&old), true);
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
    let rec = imported_product(&draft(), Some(&old), true);
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
    assert_eq!(imported_product(&draft(), None, true)["bom"], Value::Null);
}

// ── G10: a re-import merges (audit D5, D6, D17) ─────────────────────────────

/// The stored dish the audit's probe used: stopped for an empty keg, with a
/// description, in its own category, made by hand in the console.
fn stopped() -> Value {
    json!({"id": "sake-nigiri", "categoryId": "sushi", "name": "Sake Nigiri", "price": 800,
        "description": "salmon over rice", "available": false, "unavailableNote": "keg empty",
        "sortOrder": 40, "allergens": ["fish"]})
}

/// D5: a two-column price sheet changes the price and NOTHING it has no
/// column for -- the dish stays stopped, keeps its note, its text and its
/// category. Before, it went back on sale with a blank description in "menu".
#[test]
fn a_price_sheet_keeps_the_stop_the_text_and_the_category() {
    let d = dowiz_hub::import::from_csv("name,price\nSake Nigiri,900\n");
    let mut p = d.products[0].clone();
    p.id = "sake-nigiri".into();
    let rec = imported_product(&p, Some(&stopped()), d.category_column);
    assert_eq!(rec["price"], json!(900), "the file's own column wins");
    assert_eq!(rec["available"], json!(false), "still stopped");
    assert_eq!(rec["unavailableNote"], json!("keg empty"));
    assert_eq!(rec["description"], json!("salmon over rice"));
    assert_eq!((rec["categoryId"].clone(), rec["sortOrder"].clone()), (json!("sushi"), json!(40)));
    // TWIN: a NEW dish from the same sheet gets the defaults a row always had.
    let fresh = imported_product(&d.products[0], None, d.category_column);
    assert_eq!((fresh["available"].clone(), fresh["description"].clone()), (json!(true), json!("")));
    assert_eq!(fresh["categoryId"], json!("menu"));
}

/// D6: the console's "sake-nigiri" and the importer's "sushi-sake-nigiri" are
/// one dish. The row takes the console's id, so there is no twin to add and
/// no original for `retire` to stop.
#[test]
fn a_row_takes_the_id_of_the_dish_of_the_same_name() {
    let owned = [
        ("sake-nigiri".to_string(), "Sake Nigiri".to_string(), "sushi".to_string()),
        ("ebi".to_string(), "Ebi".to_string(), "sushi".to_string()),
    ];
    let mut d = dowiz_hub::import::from_csv("category,name,price\nSushi,Sake Nigiri,900\nSushi,Tamago,500\n");
    assert_eq!(d.products[0].id, "sushi-sake-nigiri", "the importer's own scheme, before");
    resolve_ids(&mut d, &owned);
    assert_eq!(d.products[0].id, "sake-nigiri", "matched by name");
    assert_eq!(d.products[1].id, "sushi-tamago", "TWIN: a new dish keeps the importer's id");
    // An id already in the catalogue is never re-pointed.
    let mut d = dowiz_hub::import::from_csv("id,name,price\nebi,Sake Nigiri,900\n");
    resolve_ids(&mut d, &owned);
    assert_eq!(d.products[0].id, "ebi");
    // Two dishes with one name: ambiguous, left alone, and said.
    let two = [owned[0].clone(), ("sake-nigiri-2".to_string(), "Sake Nigiri".to_string(), "bar".to_string())];
    let mut d = dowiz_hub::import::from_csv("name,price\nSake Nigiri,900\n");
    resolve_ids(&mut d, &two);
    assert_eq!(d.products[0].id, "menu-sake-nigiri");
    assert!(d.warnings.iter().any(|w| w.contains("matches 2 dishes")), "{:?}", d.warnings);
    // ...unless the file's category says which.
    let mut d = dowiz_hub::import::from_csv("category,name,price\nBar,Sake Nigiri,900\n");
    d.products[0].category_id = "bar".into();
    resolve_ids(&mut d, &two);
    assert_eq!(d.products[0].id, "sake-nigiri-2");
}

/// D17: an undeclared dish is held off sale on the way in; a declared one --
/// "none of the fourteen" included -- and a dish already off sale are not.
#[test]
fn an_undeclared_dish_is_held_off_sale() {
    let mut rec = json!({"id": "x", "available": true});
    assert!(hold_undeclared(&mut rec));
    assert_eq!((rec["available"].clone(), rec["unavailableNote"].clone()), (json!(false), json!(UNDECLARED_NOTE)));
    for declared in [json!(["fish"]), json!([])] {
        let mut rec = json!({"id": "x", "available": true, "allergens": declared});
        assert!(!hold_undeclared(&mut rec), "declared is not held");
        assert_eq!(rec["available"], json!(true));
    }
    let mut off = json!({"id": "x", "available": false, "unavailableNote": "keg empty"});
    assert!(!hold_undeclared(&mut off));
    assert_eq!(off["unavailableNote"], json!("keg empty"), "an existing reason is kept");
}

/// D17 on import, and its limit: the gate holds what the FILE puts on sale --
/// a new dish, or a row saying "yes" -- and leaves alone an undeclared dish a
/// price-only sheet never asked about (stopping it is the F1 failure).
#[test]
fn the_import_holds_only_what_the_file_puts_on_sale() {
    let undeclared = || json!({"id": "x", "available": true});
    let mut r = undeclared();
    assert!(hold_on_import(&mut r, true, None), "a new dish lands on sale: held");
    assert_eq!(r["available"], json!(false));
    let mut r = undeclared();
    assert!(hold_on_import(&mut r, false, Some(true)), "the file says yes: held");
    let mut r = undeclared();
    assert!(!hold_on_import(&mut r, false, None), "a price sheet asks nothing about sale");
    assert_eq!(r["available"], json!(true), "left as the owner has it");
    let mut r = json!({"id": "x", "available": true, "allergens": []});
    assert!(!hold_on_import(&mut r, true, Some(true)), "declared is never held");
}
