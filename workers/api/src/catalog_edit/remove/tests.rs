//! W-NOM: dishes deleted for good, and a category with its dishes only once
//! their number was confirmed.
use super::*;

fn catalogue() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    cat.set_location(&json!({ "id": "v", "menu_version": 3 }).to_string());
    cat.set_category("rolls", &json!({ "id": "rolls", "name": "Rolls" }).to_string());
    cat.set_category("empty", &json!({ "id": "empty", "name": "Empty" }).to_string());
    for (id, c) in [("sake", "rolls"), ("ebi", "rolls"), ("tea", "drinks")] {
        cat.set_product(id, &json!({ "id": id, "categoryId": c, "name": id, "price": 100 }).to_string());
    }
    cat
}

fn version(cat: &Catalog) -> i64 {
    serde_json::from_str::<Value>(&cat.location().unwrap()).unwrap()["menu_version"].as_i64().unwrap()
}

#[test]
fn many_dishes_go_in_one_write_and_twice_finds_nothing() {
    let mut cat = catalogue();
    let gone = remove_products(&mut cat, &["sake".into(), " tea ".into(), "sake".into(), "ghost".into(), "".into()]);
    assert_eq!(gone, vec!["sake".to_string(), "tea".to_string()]);
    assert!(cat.product("sake").is_none() && cat.product("tea").is_none());
    assert!(cat.product("ebi").is_some(), "only what was named");
    assert_eq!(version(&cat), 4, "the menu version moves once");
    assert!(remove_products(&mut cat, &["sake".into()]).is_empty(), "deleting twice is a clean nothing");
    assert_eq!(version(&cat), 4, "and a no-op does not move the version");
}

#[test]
fn a_category_with_dishes_goes_only_with_their_confirmed_count() {
    let mut cat = catalogue();
    assert_eq!(remove_category(&mut cat, "rolls", None), CategoryOut::NotEmpty(2));
    assert_eq!(remove_category(&mut cat, "rolls", Some(1)), CategoryOut::NotEmpty(2), "a stale count is refused");
    assert!(cat.product("sake").is_some() && cat.categories().iter().any(|(c, _)| c == "rolls"), "refusals delete nothing");
    let CategoryOut::Gone(mut dishes) = remove_category(&mut cat, "rolls", Some(2)) else { panic!("confirmed") };
    dishes.sort();
    assert_eq!(dishes, vec!["ebi".to_string(), "sake".to_string()]);
    assert!(cat.product("sake").is_none() && cat.product("ebi").is_none());
    assert!(cat.product("tea").is_some(), "another category's dish stays");
    assert_eq!(remove_category(&mut cat, "rolls", Some(2)), CategoryOut::Unknown, "twice: a 404");
    assert_eq!(remove_category(&mut cat, "empty", None), CategoryOut::Gone(vec![]), "an empty one needs no count");
}
