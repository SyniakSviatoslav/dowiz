//! W-CRUD: the rename, the category move and the up/down order, on the real
//! catalogue. RED on 2026-09-29: none of the three existed, the route answered
//! `ok` and changed nothing.
use super::*;

/// Three dishes in Rolls at the importer's sortOrder 0, one in Drinks.
fn catalogue() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    cat.set_category("rolls", &json!({ "id": "rolls", "name": "Rolls", "sortOrder": 20 }).to_string());
    cat.set_category("drinks", &json!({ "id": "drinks", "name": "Drinks", "sortOrder": 10 }).to_string());
    cat.set_category("empty", &json!({ "id": "empty", "name": "Empty" }).to_string());
    for (id, c) in [("a", "rolls"), ("b", "rolls"), ("c", "rolls"), ("tea", "drinks")] {
        cat.set_product(id, &json!({ "id": id, "categoryId": c, "name": id, "price": 100, "sortOrder": 0 }).to_string());
    }
    cat
}

fn product(cat: &Catalog, id: &str) -> Value {
    serde_json::from_str(&cat.product(id).unwrap()).unwrap()
}

fn order_of(cat: &Catalog, category: &str) -> Vec<String> {
    dishes_in_order(cat, category).into_iter().map(|(id, _)| id).collect()
}

#[test]
fn a_dish_is_renamed_in_the_venues_own_words_and_a_blank_name_is_refused() {
    let mut p = json!({ "id": "a", "name": "Sake", "description": "old", "price": 100 });
    rename(&mut p, Some("  Sake Nigiri "), Some(" two pieces ")).unwrap();
    assert_eq!(p["name"], "Sake Nigiri");
    assert_eq!(p["description"], "two pieces");
    rename(&mut p, None, Some("")).unwrap();
    assert_eq!(p["name"], "Sake Nigiri", "an absent name leaves the name alone");
    assert_eq!(p["description"], "", "an empty description is 'no description', not a refusal");
    assert!(rename(&mut p, Some("   "), None).unwrap_err().contains("needs a name"));
    assert!(rename(&mut p, Some(&"x".repeat(NAME_MAX + 1)), None).unwrap_err().contains("at most"));
    assert!(rename(&mut p, None, Some(&"y".repeat(DESCRIPTION_MAX + 1))).unwrap_err().contains("at most"));
    assert_eq!(p["name"], "Sake Nigiri", "a refusal writes nothing");
    assert_eq!(p["price"], 100, "the rest of the record is untouched");
}

#[test]
fn a_dish_moves_to_a_known_category_and_lands_last_there() {
    let cat = catalogue();
    let mut p = product(&cat, "tea");
    assert!(move_to_category(&cat, &mut p, "ghost").unwrap_err().starts_with("unknown category"));
    assert_eq!(p["categoryId"], "drinks", "a refusal moves nothing");
    move_to_category(&cat, &mut p, "rolls").unwrap();
    assert_eq!(p["categoryId"], "rolls");
    assert_eq!(p["sortOrder"], SORT_STEP, "after the rolls' highest sortOrder (0)");
    let mut q = product(&cat, "a");
    move_to_category(&cat, &mut q, "empty").unwrap();
    assert_eq!((q["categoryId"].as_str(), q["sortOrder"].as_i64()), (Some("empty"), Some(SORT_STEP)), "an EMPTY category can receive a dish");
    let before = product(&cat, "b");
    let mut same = before.clone();
    move_to_category(&cat, &mut same, "rolls").unwrap();
    assert_eq!(same, before, "moving to its own category changes nothing");
}

#[test]
fn dishes_move_up_and_down_inside_their_category_and_the_others_stand_still() {
    let mut cat = catalogue();
    assert_eq!(order_of(&cat, "rolls"), ["a", "b", "c"], "ties at 0 are broken by id, the storefront's order");
    assert_eq!(nudge_product(&mut cat, "c", Move::Up).unwrap(), ["a", "c", "b"]);
    assert_eq!(order_of(&cat, "rolls"), ["a", "c", "b"]);
    assert_eq!(product(&cat, "a")["sortOrder"], 10);
    assert_eq!(product(&cat, "c")["sortOrder"], 20);
    assert_eq!(product(&cat, "b")["sortOrder"], 30);
    assert_eq!(product(&cat, "tea")["sortOrder"], 0, "another category is not renumbered");
    assert_eq!(nudge_product(&mut cat, "a", Move::Up).unwrap(), ["a", "c", "b"], "the first dish moved up stays first");
    assert_eq!(nudge_product(&mut cat, "b", Move::Down).unwrap(), ["a", "c", "b"], "the last moved down stays last");
    assert_eq!(nudge_product(&mut cat, "a", Move::Down).unwrap(), ["c", "a", "b"]);
    assert!(nudge_product(&mut cat, "ghost", Move::Up).unwrap_err().starts_with("unknown product"));
    assert_eq!(order_of(&cat, "rolls"), ["c", "a", "b"], "a refusal writes nothing");
}

#[test]
fn categories_move_up_and_down_too() {
    let mut cat = catalogue();
    let ids = |cat: &Catalog| categories_in_order(cat).into_iter().map(|(id, _)| id).collect::<Vec<_>>();
    assert_eq!(ids(&cat), ["empty", "drinks", "rolls"], "a category without sortOrder reads as 0");
    assert_eq!(nudge_category(&mut cat, "rolls", Move::Up).unwrap(), ["empty", "rolls", "drinks"]);
    assert_eq!(ids(&cat), ["empty", "rolls", "drinks"]);
    assert_eq!(nudge_category(&mut cat, "empty", Move::Down).unwrap(), ["rolls", "empty", "drinks"]);
    let rolls: Value = serde_json::from_str(&cat.categories().into_iter().find(|(c, _)| c == "rolls").unwrap().1).unwrap();
    assert_eq!(rolls["name"], "Rolls", "the record keeps its words");
    assert_eq!(rolls["sortOrder"], 10);
    assert!(nudge_category(&mut cat, "ghost", Move::Down).unwrap_err().starts_with("unknown category"));
}

#[test]
fn the_wire_words_are_up_and_down_and_nothing_else() {
    assert_eq!(Move::from_wire(" up "), Ok(Move::Up));
    assert_eq!(Move::from_wire("down"), Ok(Move::Down));
    assert!(Move::from_wire("left").unwrap_err().contains("not a move"));
    assert_eq!(nudged(&["x".into(), "y".into()], "y", Move::Up).unwrap(), ["y", "x"]);
    assert!(nudged(&["x".into()], "z", Move::Up).is_err());
}
