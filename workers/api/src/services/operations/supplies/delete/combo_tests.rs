//! W-VERIFY (2026-09-30): delete and edit COMBINATIONS of raw item, ПФ and
//! dish -- the class that already produced one resurrection (43aae639). After
//! each sequence the catalogue must be CONSISTENT: every card and recipe line
//! names a supply that exists, no card is empty, no deleted id is back, every
//! dish's stored cost is what a fresh derivation says, and the preps list
//! names only dishes and cards that exist.
use super::*;
use crate::services::operations::preps::{self, LineIn, PrepIn};
use crate::services::operations::preps::rederive::lines_of;

fn catalogue() -> Catalog {
    let mut cat = Catalog::create().unwrap();
    for (id, unit, cost) in [("rice", "g", 38), ("vinegar", "ml", 20), ("salt", "g", 5)] {
        cat.set_supply(id, &json!({ "id": id, "name": id, "unit": unit, "kind": "food_ingredient", "costPerBasis": cost }).to_string());
    }
    save_prep(&mut cat, "mitsukan", &[("vinegar", 800), ("salt", 50)], 850);
    save_prep(&mut cat, "sushi-rice", &[("rice", 1000), ("mitsukan", 250)], 1200);
    for (id, lines) in [("roll", vec![("sushi-rice", 130), ("salt", 1)]), ("bowl", vec![("rice", 200)])] {
        let mut d = json!({ "id": id, "name": id, "price": 500, "categoryId": "c1" });
        let l: Vec<BomLineIn> = lines.iter().map(|(s, q)| BomLineIn { supply: s.to_string(), qty: *q, net: None, out: None }).collect();
        set_bom(&mut d, &l, |s| cat.supply(s), Typed::default()).unwrap();
        cat.set_product(id, &d.to_string());
    }
    cat
}

fn save_prep(cat: &mut Catalog, id: &str, lines: &[(&str, i64)], yield_qty: i64) {
    let body = PrepIn { id: id.into(), lines: lines.iter().map(|(i, q)| LineIn { item: i.to_string(), qty: *q }).collect(), yield_qty, ..PrepIn::default() };
    let (id, card) = preps::check(&body).unwrap();
    preps::save(cat, &id, &body, &card).unwrap();
}

fn consistent(cat: &Catalog, gone: &[&str]) {
    for g in gone {
        assert!(cat.supply(g).is_none() && cat.product(g).is_none(), "{g} came back");
    }
    for (sid, j) in cat.supplies() {
        if let Some(c) = dowiz_hub::prep::card_of(&j) {
            assert!(!c.lines.is_empty(), "{sid}: empty card");
            for l in c.lines {
                assert!(cat.supply(&l.item).is_some(), "{sid}'s card names missing {}", l.item);
            }
        }
    }
    for (pid, j) in cat.products() {
        let p: Value = serde_json::from_str(&j).unwrap();
        let lines = lines_of(&p);
        for l in &lines {
            assert!(cat.supply(&l.supply).is_some(), "{pid}'s recipe names missing {}", l.supply);
        }
        let mut fresh = p.clone();
        set_bom(&mut fresh, &lines, |s| cat.supply(s), Typed::from_record(&p)).unwrap();
        assert_eq!(p["cost"], fresh["cost"], "{pid}: stored cost is stale");
        assert_eq!(p["weightG"], fresh["weightG"], "{pid}: stored weight is stale");
    }
    for v in preps::list(cat) {
        for d in v["uses"]["dishes"].as_array().unwrap() {
            assert!(cat.product(d["id"].as_str().unwrap()).is_some(), "preps list names a deleted dish");
        }
    }
}

#[test]
fn raw_prep_and_dish_deleted_together_in_every_order() {
    let ids = ["salt", "mitsukan", "roll"];
    for order in [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]] {
        let mut cat = catalogue();
        for i in order {
            if ids[i] == "roll" {
                cat.remove_product("roll");
            } else {
                delete_in(&mut cat, &[ids[i].to_string()], true).unwrap();
            }
        }
        consistent(&cat, &ids);
    }
    let mut cat = catalogue();
    delete_in(&mut cat, &["salt".into(), "mitsukan".into()], true).unwrap();
    cat.remove_product("roll");
    consistent(&cat, &ids);
}

#[test]
fn a_raw_item_whose_deletion_would_empty_a_card_waits_for_the_card() {
    let mut cat = catalogue();
    delete_in(&mut cat, &["salt".into()], true).unwrap(); // mitsukan keeps vinegar
    assert!(matches!(delete_in(&mut cat, &["vinegar".into()], true), Err(Held::Emptied(e)) if e == vec!["mitsukan".to_string()]));
    delete_in(&mut cat, &["vinegar".into(), "mitsukan".into()], true).unwrap();
    consistent(&cat, &["salt", "vinegar", "mitsukan"]);
    // sushi-rice keeps rice; the roll still derives through it.
    assert!(cat.product("roll").unwrap().contains("sushi-rice"));
}

#[test]
fn rename_retire_and_edit_then_delete() {
    let mut cat = catalogue();
    // Rename (a save with a new name) keeps the id; the roll follows.
    let body = PrepIn { id: "mitsukan".into(), name: Some("Mitsukan 2".into()), lines: vec![LineIn { item: "vinegar".into(), qty: 8000 }, LineIn { item: "salt".into(), qty: 50 }], yield_qty: 850, ..PrepIn::default() };
    let (id, card) = preps::check(&body).unwrap();
    let (_, dishes) = preps::save(&mut cat, &id, &body, &card).unwrap();
    assert_eq!(dishes, vec!["roll".to_string()], "an edited card re-derives the dish through two cards");
    consistent(&cat, &[]);
    // Retire, then delete.
    let mut v: Value = serde_json::from_str(&cat.supply("mitsukan").unwrap()).unwrap();
    v["active"] = json!(false);
    cat.set_supply("mitsukan", &v.to_string());
    delete_in(&mut cat, &["mitsukan".into()], true).unwrap();
    consistent(&cat, &["mitsukan"]);
    // Edit a card while a dish uses it, then delete the dish: the ПФ stays, unused.
    save_prep(&mut cat, "sushi-rice", &[("rice", 1100)], 1300);
    cat.remove_product("roll");
    consistent(&cat, &["roll", "mitsukan"]);
    let list = preps::list(&cat);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0]["uses"]["dishes"], json!([]));
}
