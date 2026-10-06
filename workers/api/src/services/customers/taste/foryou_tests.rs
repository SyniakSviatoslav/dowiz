//! W-TASTE2 row 2, pinned: the order page's "For you" from the venue's profile of the guest --
//! nothing after an objection, nothing without a profile or without taste in it, the dishes of the
//! taste block in its own order with the card's allergens and this order's dishes left out, and no
//! number in the answer.

use super::*;
use crate::services::customers::taste::{apply_order, Line, SyncIn};
use dowiz_hub::block::encode::encode;
use serde_json::json;

const D: i64 = 20_000;

fn dish(id: &str, sense: Value, allergens: Value, on_sale: bool) -> (String, String) {
    (id.to_string(), json!({ "id": id, "name": id, "sense": sense, "allergens": allergens, "available": on_sale }).to_string())
}

/// Five dishes: two smoky-crispy, one with fish, one sweet-soft, one smoky but off sale.
fn block() -> Vec<u8> {
    let smoky = json!({"taste": {"salty": 3}, "texture": {"crispy": 3}, "aroma": {"smoky": 3}});
    let menu = vec![
        dish("ebi-tempura", smoky.clone(), json!([]), true),
        dish("unagi", json!({"taste": {"salty": 2}, "texture": {"crispy": 2}, "aroma": {"smoky": 2}}), json!([]), true),
        dish("salmon-aburi", smoky.clone(), json!(["fish"]), true),
        dish("mochi", json!({"taste": {"sweet": 5}, "texture": {"soft": 3}}), json!([]), true),
        dish("smoked-off", smoky, json!([]), false),
    ];
    let (b, skipped) = dowiz_hub::block::taste::project(&menu).unwrap();
    assert!(skipped.is_empty());
    encode(&b).unwrap()
}

fn smoky_guest() -> Profile {
    let s = dowiz_hub::sense::validate(&json!({"taste": {"salty": 3}, "texture": {"crispy": 3}, "aroma": {"smoky": 3}})).unwrap();
    apply_order(None, &[Line { qty: 2, sense: dowiz_hub::sense::vector(&s), ..Line::default() }], None, D)
}

fn ids(v: &Value) -> Vec<String> {
    v["items"].as_array().unwrap().iter().map(|x| x["id"].as_str().unwrap().to_string()).collect()
}

#[test]
fn a_guest_with_a_profile_gets_the_blocks_top_dishes_without_this_order_or_off_sale() {
    let b = block();
    let p = smoky_guest();
    let v = answer(false, Some(&p), Some(&b), &[], &[]).unwrap();
    assert_eq!(v["state"], "shown", "{v}");
    assert_eq!(v["contract"], CONTRACT);
    assert_eq!(ids(&v), vec!["ebi-tempura", "salmon-aburi", "unagi"], "best first, ties by id; the off-sale dish never: {v}");
    let after = answer(false, Some(&p), Some(&b), &[], &["ebi-tempura".to_string()]).unwrap();
    assert_eq!(ids(&after), vec!["salmon-aburi", "unagi"], "the dish just ordered is left out: {after}");
    assert!(!ids(&v).contains(&"mochi".to_string()), "a dish that shares no taste is not suggested");
    // The answer explains in the guest's own words and carries no number at all.
    assert_eq!(v["because"], json!(["a:smoky", "x:crispy"]));
    for x in v["items"].as_array().unwrap() {
        assert_eq!(x.as_object().unwrap().keys().collect::<Vec<_>>(), vec!["id"], "ids only: {x}");
    }
}

#[test]
fn the_cards_allergens_and_undeclared_dishes_are_left_out_before_any_ranking() {
    let b = block();
    let p = smoky_guest();
    let v = answer(false, Some(&p), Some(&b), &["fish".to_string()], &[]).unwrap();
    assert_eq!(ids(&v), vec!["ebi-tempura", "unagi"], "{v}");
    // Positive twin: no allergen on the card, the fish dish is back.
    assert!(ids(&answer(false, Some(&p), Some(&b), &[], &[]).unwrap()).contains(&"salmon-aburi".to_string()));
}

#[test]
fn an_objection_no_profile_or_no_taste_shows_nothing_and_says_which() {
    let b = block();
    let p = smoky_guest();
    let off = answer(true, Some(&p), Some(&b), &[], &[]).unwrap();
    assert_eq!((off["state"].clone(), ids(&off).len()), (json!("off"), 0), "objected: {off}");
    let none = answer(false, None, Some(&b), &[], &[]).unwrap();
    assert_eq!((none["state"].clone(), ids(&none).len()), (json!("no-profile"), 0));
    // Ordered only dishes that declare nothing: a profile, but no axes.
    let bare = apply_order(None, &[Line { qty: 1, ..Line::default() }], None, D);
    let nt = answer(false, Some(&bare), Some(&b), &[], &[]).unwrap();
    assert_eq!((nt["state"].clone(), ids(&nt).len()), (json!("no-taste"), 0));
    // No block (the catalogue did not fit one): shown, but empty -- never a guess from elsewhere.
    let nb = answer(false, Some(&p), None, &[], &[]).unwrap();
    assert_eq!((nb["state"].clone(), ids(&nb).len()), (json!("shown"), 0));
}

#[test]
fn a_profile_with_only_the_phones_vector_ranks_by_it_and_a_broken_block_is_an_error_not_a_guess() {
    let b = block();
    let mut p = apply_order(None, &[Line { qty: 1, ..Line::default() }], None, D);
    p.device = Some(SyncIn { v: 1, sense: [("t:sweet".to_string(), 1000), ("x:soft".to_string(), 600)].into_iter().collect(), ..SyncIn::default() });
    let v = answer(false, Some(&p), Some(&b), &[], &[]).unwrap();
    assert_eq!(ids(&v), vec!["mochi"], "{v}");
    assert_eq!(v["because"], json!([]), "the venue's own words only come from the venue's own weights");
    let mut broken = b.clone();
    let n = broken.len();
    broken[n - 1] ^= 0xff;
    assert!(answer(false, Some(&p), Some(&broken), &[], &[]).is_err(), "a corrupt block is said, not ranked");
}
