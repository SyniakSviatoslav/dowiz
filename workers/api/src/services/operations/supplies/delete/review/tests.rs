//! NO LOSS AROUND A DELETION (2026-10-16): an option's recipe is a use the
//! owner confirms; a card ABOVE a card that lost a line is marked too; and
//! saving the card a dish's mark points through (`via`) clears THAT mark.

use super::super::*;
use crate::recipe::apply::set_bom;
use crate::services::operations::preps::{self, LineIn, PrepIn};

fn raw(cat: &mut Catalog, id: &str) {
    cat.set_supply(id, &json!({ "id": id, "name": id, "unit": "g", "kind": "food_ingredient", "costPerBasis": 10 }).to_string());
}

fn save_prep(cat: &mut Catalog, id: &str, lines: &[(&str, i64)], yield_qty: i64) {
    let body = PrepIn { id: id.into(), lines: lines.iter().map(|(i, q)| LineIn { item: i.to_string(), qty: (*q).into() }).collect(), yield_qty: yield_qty.into(), ..PrepIn::default() };
    let (id, card) = preps::check(&body).unwrap();
    preps::save(cat, &id, &body, &card).unwrap();
}

fn rec(j: Option<String>) -> Value {
    serde_json::from_str(&j.expect("record")).unwrap()
}

/// Item 4: salmon only in an option ("extra salmon") is IN USE.
#[test]
fn a_supply_used_only_by_an_option_is_in_use_and_listed() {
    let mut cat = Catalog::create().unwrap();
    raw(&mut cat, "salmon");
    raw(&mut cat, "rice");
    let dish = json!({ "id": "roll", "name": "Roll", "price": 500, "modifierGroups": [{ "id": "g", "name": "Extra", "options": [{ "id": "xs", "name": "Extra salmon" }] }],
        "optionBom": { "xs": [{ "supply": "salmon", "qty": 20 }] } });
    cat.set_product("roll", &dish.to_string());
    let used = match delete_in(&mut cat, &["salmon".into()], false) {
        Err(Held::InUse(v)) => v,
        other => panic!("a supply an option draws must be confirmed first: {other:?}"),
    };
    assert_eq!(used["salmon"]["options"], json!([{ "id": "roll", "name": "Roll", "option": "xs" }]));
    assert!(cat.supply("salmon").is_some(), "nothing removed before the owner confirms");
    // GET /supplies/salmon/uses lists it too.
    let a = preps::answer(&cat, "uses:salmon").unwrap();
    assert_eq!(a["uses"]["options"], json!([{ "id": "roll", "name": "Roll", "option": "xs" }]), "{a}");
    // TWIN: rice, used nowhere, deletes without asking; and its uses list no options key.
    assert_eq!(preps::answer(&cat, "uses:rice").unwrap()["uses"], json!({ "preps": [], "dishes": [] }));
    assert!(delete_in(&mut cat, &["rice".into()], false).is_ok());
    // Confirmed: the option loses the line and the dish is marked naming it.
    let r = delete_in(&mut cat, &["salmon".into()], true).unwrap();
    assert_eq!(r.dishes, vec!["roll".to_string()]);
    assert_eq!(rec(cat.product("roll"))["recipeReview"][0]["option"], json!("xs"));
}

/// A <- B <- C: mitsukan loses salt; sushi-rice names mitsukan; big-rice
/// names sushi-rice. Both above are marked, with `via` the card that lost it.
fn tower() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    for id in ["vinegar", "salt", "rice"] {
        raw(&mut cat, id);
    }
    save_prep(&mut cat, "mitsukan", &[("vinegar", 800), ("salt", 50)], 850);
    save_prep(&mut cat, "sushi-rice", &[("rice", 1000), ("mitsukan", 250)], 1200);
    save_prep(&mut cat, "big-rice", &[("sushi-rice", 1000)], 1000);
    let mut d = json!({ "id": "roll", "name": "Roll", "price": 500 });
    let l = [("sushi-rice", 130), ("salt", 1)].map(|(s, q)| BomLineIn { supply: s.into(), qty: q.into(), net: None, out: None });
    set_bom(&mut d, &l, |s| cat.supply(s), Typed::default()).unwrap();
    cat.set_product("roll", &d.to_string());
    cat
}

/// Item 5.
#[test]
fn a_card_above_a_card_that_lost_a_line_is_marked_transitively() {
    let mut cat = tower();
    let r = remove_supplies(&mut cat, &["salt".into()]);
    let a = rec(cat.supply("mitsukan"));
    assert_eq!(a["review"], json!([{ "supply": "salt", "name": "salt", "qty": 50 }]));
    let want = json!([{ "supply": "salt", "name": "salt", "qty": 50, "via": "mitsukan" }]);
    assert_eq!(rec(cat.supply("sushi-rice"))["review"], want, "the card above");
    assert_eq!(rec(cat.supply("big-rice"))["review"], want, "and the one above it");
    let mut marked: Vec<&str> = r.review.iter().filter(|v| v["prep"] == json!(true)).filter_map(|v| v["id"].as_str()).collect();
    marked.sort();
    assert_eq!(marked, vec!["big-rice", "mitsukan", "sushi-rice"], "each answered once");
    // Saving B clears B's mark, and only B's.
    save_prep(&mut cat, "sushi-rice", &[("rice", 1000), ("mitsukan", 250)], 1200);
    assert!(rec(cat.supply("sushi-rice")).get("review").is_none());
    assert_eq!(rec(cat.supply("mitsukan"))["review"], a["review"]);
    // TWIN: a card not above the touched one is not marked.
    let mut cat = tower();
    raw(&mut cat, "sugar");
    save_prep(&mut cat, "syrup", &[("sugar", 100)], 100);
    remove_supplies(&mut cat, &["salt".into()]);
    assert!(rec(cat.supply("syrup")).get("review").is_none());
}

/// Item 6.
#[test]
fn saving_the_card_a_mark_points_through_clears_that_mark_only() {
    let mut cat = tower();
    remove_supplies(&mut cat, &["salt".into()]);
    let marks = rec(cat.product("roll"))["recipeReview"].clone();
    assert!(marks.as_array().unwrap().iter().any(|m| m["via"] == json!("mitsukan")), "{marks}");
    assert!(marks.as_array().unwrap().iter().any(|m| m.get("via").is_none()), "the roll named salt itself too: {marks}");
    // The owner fixes mitsukan (a new line): the marks through it go...
    raw(&mut cat, "sugar");
    save_prep(&mut cat, "mitsukan", &[("vinegar", 800), ("sugar", 50)], 850);
    let left = rec(cat.product("roll"))["recipeReview"].clone();
    assert_eq!(left, json!([{ "supply": "salt", "name": "salt", "qty": 1 }]), "...the roll's own salt line stays marked");
    assert!(rec(cat.supply("sushi-rice")).get("review").is_none(), "the card above was marked through mitsukan too");
    // TWIN: saving a card no mark points through leaves the marks.
    let mut cat = tower();
    remove_supplies(&mut cat, &["salt".into()]);
    let before = rec(cat.product("roll"))["recipeReview"].clone();
    save_prep(&mut cat, "big-rice", &[("sushi-rice", 1000)], 1000);
    assert_eq!(rec(cat.product("roll"))["recipeReview"], before);
}
