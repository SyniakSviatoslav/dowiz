//! W-NOM: a list of names becomes ingredients -- ids minted, the form's rules
//! kept, what exists skipped, one bad line refusing the whole list.
use super::*;

fn stored() -> Vec<(String, String)> {
    vec![("salmon".into(), json!({ "id": "salmon", "name": "Salmon", "unit": "g" }).to_string())]
}

#[test]
fn names_become_ingredients_with_their_unit_and_group() {
    let items = vec![
        json!({ "name": "Rice", "unit": "g", "category": "Dry" }),
        json!({ "name": "Soy sauce", "unit": "ml", "category": "Sauces", "packs": [{ "name": "bottle 1 l", "qty": 1000 }] }),
        json!({ "name": "  SALMON " }),
        json!({ "name": "Rice 2" }),
        json!({ "name": "Lids", "unit": "unit", "kind": "packaging", "barcode": "4006381333931" }),
    ];
    let mut have = stored();
    have.push(("rice-2".into(), json!({ "id": "rice-2", "name": "Basmati", "unit": "g" }).to_string()));
    let a = plan(&items, &have).unwrap();
    let ids: Vec<&str> = a.write.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["rice", "soy-sauce", "rice-2-2", "lids"], "an id another name holds is not taken: the new one gets a tail");
    assert_eq!(a.existing, vec!["SALMON".to_string()], "a name the venue has is skipped, whatever its case");
    let soy = &a.write[1].1;
    assert_eq!((soy["unit"].as_str(), soy["category"].as_str()), (Some("ml"), Some("Sauces")));
    assert_eq!(soy["packs"][0]["qty"], 1000);
    assert_eq!(soy["active"], true);
    assert_eq!(a.write[3].1["kind"], "packaging");
    assert_eq!(a.write[0].1["name"], "Rice", "the name is kept as typed, trimmed");
}

#[test]
fn one_bad_line_refuses_the_list_and_its_twin_passes() {
    for bad in [json!({ "name": "" }), json!({ "name": "X", "unit": "kg" }), json!({ "name": "X", "barcode": "12" }),
                json!({ "name": "X", "price": 5 }), json!("X"), json!({ "name": "x".repeat(81) })] {
        assert!(plan(&[json!({ "name": "Ok" }), bad.clone()], &[]).is_err(), "{bad}");
    }
    assert!(plan(&[json!({ "name": "x".repeat(80), "unit": "unit" })], &[]).is_ok());
    assert!(plan(&[], &[]).is_err(), "nothing to add is a 400");
    let many: Vec<Value> = (0..=ITEMS_MAX).map(|i| json!({ "name": format!("n{i}") })).collect();
    assert!(plan(&many, &[]).is_err());
    assert_eq!(plan(&many[..ITEMS_MAX], &[]).unwrap().write.len(), ITEMS_MAX);
}

/// The literal cases the console's resolver reads too: one list, two readers.
const CASES: &str = include_str!("../../../../../public/admin/supply-resolve-cases.json");

fn pairs(section: &str) -> Vec<(String, String)> {
    let all: Value = serde_json::from_str(CASES).unwrap();
    let rows = all[section].as_array().unwrap_or_else(|| panic!("no section {section}"));
    rows.iter().map(|r| (r[0].as_str().unwrap().to_string(), r[1].as_str().unwrap().to_string())).collect()
}

#[test]
fn the_fold_agrees_with_the_console_on_every_shared_case() {
    let (folds, same, differ) = (pairs("fold"), pairs("same"), pairs("differ"));
    assert!(folds.len() >= 20 && same.len() >= 5 && differ.len() >= 5, "the list is read, not empty");
    for (typed, want) in &folds {
        assert_eq!(&fold(typed), want, "{typed:?}");
    }
    for (a, b) in &same {
        assert_eq!(fold(a), fold(b), "{a:?} = {b:?}");
    }
    for (a, b) in &differ {
        assert_ne!(fold(a), fold(b), "{a:?} != {b:?}");
    }
    for (typed, have) in pairs("category") {
        assert_eq!(fold(&typed), fold(&have), "one group: {typed:?} and {have:?}");
    }
    for (a, b) in pairs("categoryApart") {
        assert_ne!(fold(&a), fold(&b), "two groups: {a:?} and {b:?}");
    }
    assert_eq!(fold("\u{435}\u{308}\u{43b}\u{43a}\u{430}"), fold("\u{451}\u{43b}\u{43a}\u{430}"), "base + combining mark is the composed letter");
}

fn kitchen() -> Vec<(String, String)> {
    vec![
        ("eggs".into(), json!({ "id": "eggs", "name": "Яйця", "unit": "unit", "category": "Овочі" }).to_string()),
        ("soy".into(), json!({ "id": "soy", "name": "Соус соєвий", "unit": "ml", "category": "Соуси" }).to_string()),
        ("old".into(), json!({ "id": "old", "name": "Вугор", "unit": "g", "active": false }).to_string()),
    ]
}

#[test]
fn a_name_typed_another_way_is_the_record_the_venue_has_and_the_answer_says_which() {
    let items = vec![
        json!({ "name": " ЯЙЦЯ " }),
        json!({ "name": "соус соевый", "unit": "g" }),
        json!({ "name": "яйцо", "unit": "unit" }),
        json!({ "name": "Палички", "unit": "unit", "kind": "utensil" }),
        json!({ "name": "палички!", "unit": "g" }),
        json!({ "name": "вугор" }),
    ];
    let a = plan(&items, &kitchen()).unwrap();
    let got: Vec<(&str, &str, &str, bool)> = a.matched.iter().map(|m| (m.typed.as_str(), m.id.as_str(), m.unit.as_str(), m.retired)).collect();
    assert_eq!(
        got,
        vec![("ЯЙЦЯ", "eggs", "unit", false), ("соус соевый", "soy", "ml", false), ("палички!", "палички", "unit", false), ("вугор", "old", "g", true)],
        "the stored record's id and unit, also for a name minted earlier in this list; a retired one says so"
    );
    assert_eq!(a.matched[0].name, "Яйця", "the stored spelling");
    assert_eq!(a.existing, vec!["ЯЙЦЯ", "соус соевый", "палички!", "вугор"]);
    // Twin: another ending is NOT the same name here; that question is the form's.
    let ids: Vec<&str> = a.write.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["яйцо", "палички"]);
    // Pasting the same paper again writes nothing.
    let mut have = kitchen();
    have.extend(a.write.iter().map(|(id, r)| (id.clone(), r.to_string())));
    let again = plan(&items, &have).unwrap();
    assert_eq!((again.write.len(), again.matched.len()), (0, 6));
}

#[test]
fn a_category_is_stored_under_the_spelling_the_venue_has_and_a_new_one_once() {
    let items = vec![
        json!({ "name": "Морква", "category": " овочи " }),
        json!({ "name": "Судочок 500", "unit": "unit", "kind": "packaging", "category": "Упаковка" }),
        json!({ "name": "Кришка", "unit": "unit", "kind": "packaging", "category": "упаковка" }),
        json!({ "name": "Сіль" }),
    ];
    let a = plan(&items, &kitchen()).unwrap();
    let cats: Vec<&str> = a.write.iter().map(|(_, r)| r["category"].as_str().unwrap()).collect();
    assert_eq!(cats, vec!["Овочі", "Упаковка", "Упаковка", ""], "the venue's spelling; a new group as first typed; none stays none");
    for (typed, have) in pairs("category") {
        let stored = vec![("x".to_string(), json!({ "id": "x", "name": "X", "category": have }).to_string())];
        let one = plan(&[json!({ "name": "Y", "category": typed })], &stored).unwrap();
        assert_eq!(one.write[0].1["category"], json!(have), "{typed:?}");
    }
}

/// A typed packaging item, end to end: created by the list door, put on a
/// dish's recipe, and a sale (an order settled, a till line served) takes it
/// off the shelf BY ITS ID.
#[test]
fn a_typed_container_goes_on_a_recipe_and_a_sale_writes_it_off_by_id() {
    use crate::recipe::apply::{set_bom, Typed};
    use crate::recipe::BomLineIn;
    use dowiz_hub::stock::{draws_for, reservations_for, settle, StockLog};

    let a = plan(&[json!({ "name": "Судочок 500 мл", "unit": "unit", "kind": "packaging", "category": "Упаковка" })], &kitchen()).unwrap();
    let (id, rec) = a.write[0].clone();
    assert_eq!((rec["kind"].as_str(), rec["unit"].as_str()), (Some("packaging"), Some("unit")));
    assert!(rec["kcalPer100"].is_null(), "no nutrition is asked of a container");
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_supply(&id, &rec.to_string());
    let mut soup = json!({ "id": "soup", "name": "Miso", "price": 300 });
    set_bom(&mut soup, &[BomLineIn { supply: id.clone(), qty: 1i64.into(), net: None, out: None }], |s| cat.supply(s), Typed::default()).unwrap();
    assert_eq!(soup["bom"][0]["supply"], json!(id), "the recipe names it by id");
    let dish = soup.to_string();

    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with(&id, 100, &Default::default()).unwrap();
    log.append_all(&reservations_for("o1", &[(dish.clone(), 3)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", true)).unwrap();
    assert_eq!(log.ledger().unwrap().level(&id).on_hand, 97, "an order of 3 takes 3 containers");
    log.append_served_draws(&draws_for("till-1", &[(dish, 2)])).unwrap();
    assert_eq!(log.ledger().unwrap().level(&id).on_hand, 95, "a till sale of the dish takes 2 more");
    // Twin: a dish without the line takes none.
    log.append_served_draws(&draws_for("till-2", &[(json!({ "id": "tea", "name": "Tea" }).to_string(), 4)])).unwrap();
    assert_eq!(log.ledger().unwrap().level(&id).on_hand, 95);
}
