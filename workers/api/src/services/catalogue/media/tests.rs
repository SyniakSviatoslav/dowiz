//! W-CRUD: a photograph set or cleared is a menu change -- the version moves,
//! so a cart notices, and both renderings go when the owner removes it.
use super::*;
use dowiz_hub::catalog::Catalog;

fn catalogue() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    cat.set_location(&json!({ "id": "v", "menu_version": 3 }).to_string());
    cat.set_product("sake", &json!({ "id": "sake", "name": "Sake", "price": 100, "imageUrl": "/media/old.png", "imageUrlSmall": "/media/old-s.png" }).to_string());
    cat
}

fn version(cat: &Catalog) -> i64 {
    serde_json::from_str::<Value>(&cat.location().unwrap()).unwrap()["menu_version"].as_i64().unwrap()
}

fn dish(cat: &Catalog) -> Value {
    serde_json::from_str(&cat.product("sake").unwrap()).unwrap()
}

#[test]
fn a_new_full_photograph_drops_the_old_card_and_moves_the_menu_version() {
    let mut cat = catalogue();
    set_image_in(&mut cat, "sake", "/media/new.png", false).unwrap();
    let p = dish(&cat);
    assert_eq!(p["imageUrl"], "/media/new.png");
    assert!(p.get("imageUrlSmall").is_none(), "the card of the replaced picture is gone");
    assert_eq!(version(&cat), 4, "RED 2026-09-29: the version did not move on a photo write");
    set_image_in(&mut cat, "sake", "/media/new-s.png", true).unwrap();
    let p = dish(&cat);
    assert_eq!((p["imageUrl"].as_str(), p["imageUrlSmall"].as_str()), (Some("/media/new.png"), Some("/media/new-s.png")), "a small variant sits beside the full one");
    assert_eq!(version(&cat), 5);
    assert!(set_image_in(&mut cat, "ghost", "/media/x.png", false).is_err());
    assert_eq!(version(&cat), 5, "a refusal moves nothing");
}

#[test]
fn clearing_takes_both_renderings_and_moves_the_menu_version() {
    let mut cat = catalogue();
    clear_image_in(&mut cat, "sake").unwrap();
    let p = dish(&cat);
    assert!(p["imageUrl"].is_null());
    assert!(p.get("imageUrlSmall").is_none());
    assert_eq!(p["name"], "Sake", "the rest of the dish stands");
    assert_eq!(version(&cat), 4, "RED 2026-09-29: menuVersion 39 -> 39 on qa-durres after a clear");
    assert!(clear_image_in(&mut cat, "ghost").is_err());
}
