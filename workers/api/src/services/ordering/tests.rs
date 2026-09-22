//! The basket's rules, each pinned against the defect it came from.
//!
//! Every test here is a case the promo preview got wrong while the checkout
//! got it right, or a case that cost a venue money on a real order.

use super::pricing::*;
use serde_json::json;

/// A catalogue: product id → record, the same shape `Catalog::product` returns.
fn menu(items: &[(&str, serde_json::Value)]) -> impl Fn(&str) -> Option<String> {
    let m: std::collections::BTreeMap<String, String> =
        items.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |id: &str| m.get(id).cloned()
}

fn dish(name: &str, price: i64) -> serde_json::Value {
    json!({ "name": name, "price": price, "available": true })
}

fn want<'a>(id: &'a str, mods: &'a [String], qty: i64) -> Want<'a> {
    Want { product_id: id, modifier_ids: mods, quantity: qty }
}

/// The salmon dish: 800 base, "extra salmon" +200, one optional group.
fn salmon() -> serde_json::Value {
    json!({
        "name": "Sake nigiri", "price": 800, "available": true,
        "modifierGroups": [{
            "id": "extras", "name": "Extras", "min": 0, "max": 2,
            "options": [
                { "id": "extra-salmon", "name": "Extra salmon", "priceDelta": 200 },
                { "id": "no-wasabi", "name": "No wasabi", "priceDelta": 0 },
                { "id": "roe", "name": "Roe", "priceDelta": 150, "available": false }
            ]
        }]
    })
}

/// A size group that MUST be chosen from.
fn ramen() -> serde_json::Value {
    json!({
        "name": "Ramen", "price": 1000, "available": true,
        "modifierGroups": [{
            "id": "size", "name": "Size", "min": 1, "max": 1,
            "options": [
                { "id": "regular", "name": "Regular", "priceDelta": 0 },
                { "id": "large", "name": "Large", "priceDelta": 300 }
            ]
        }]
    })
}

/// THE OPTIONS ARE PART OF THE PRICE. A dish at 800 with "extra salmon +200",
/// ordered twice, was shown as 2000 by the cart and stored and charged at 1600
/// — the venue paid the difference on every order with a paid option.
#[test]
fn an_options_delta_is_in_the_unit_price_and_therefore_in_the_line() {
    let cat = menu(&[("sake", salmon())]);
    let mods = vec!["extra-salmon".to_string()];
    let b = price_basket(&cat, [want("sake", &mods, 2)]).expect("a valid selection");
    assert_eq!(b.lines[0].unit_price, 1000, "800 base + 200 option");
    assert_eq!(b.subtotal, 2000, "and the quantity multiplies the WHOLE unit price");
    assert_eq!(b.lines[0].name, "Sake nigiri", "the name travels with the line");
}

/// A SELECTION THE CHECKOUT REFUSES MUST NOT BE QUOTED. The preview did
/// `price(..).map(|c| c.delta).unwrap_or(0)`, so an id from another dish, a
/// required group left empty and a sold-out extra were all priced at the
/// dish's base price and discounted.
#[test]
fn a_refused_selection_is_a_refusal_and_never_a_zero_delta() {
    let cat = menu(&[("sake", salmon()), ("ramen", ramen())]);

    let stale = vec!["from-another-dish".to_string()];
    let e = price_basket(&cat, [want("sake", &stale, 1)]).unwrap_err();
    assert_eq!(e.status(), 400);
    assert!(e.text().starts_with("sake: "), "the dish is named: {}", e.text());

    let none = Vec::new();
    let e = price_basket(&cat, [want("ramen", &none, 1)]).unwrap_err();
    assert_eq!(e, Refusal::Options { product_id: "ramen".into(), why: "choose 1 from \"Size\"".into() });

    let out = vec!["roe".to_string()];
    let e = price_basket(&cat, [want("sake", &out, 1)]).unwrap_err();
    assert!(e.text().contains("run out"), "{}", e.text());
}

/// A DISH THE KITCHEN TURNED OFF WAS PRICED AND DISCOUNTED by the preview,
/// and the checkout then refused the order the customer had been quoted for.
/// A record with no `available` field is one nobody finished, and serving it
/// is worse than refusing it.
#[test]
fn an_unavailable_dish_is_refused_and_so_is_one_that_never_said() {
    let cat = menu(&[
        ("off", json!({ "name": "Off", "price": 500, "available": false })),
        ("silent", json!({ "name": "Silent", "price": 500 })),
    ]);
    let none = Vec::new();
    for id in ["off", "silent"] {
        let e = price_basket(&cat, [want(id, &none, 1)]).unwrap_err();
        assert_eq!(e, Refusal::Unavailable(id.into()));
        assert_eq!(e.status(), 409, "the kitchen changed its mind, the browser was not wrong");
    }
}

/// A DISH WITH NO PRICE COUNTED AS FREE. That is not only a free dish: the
/// subtotal is what a promo's minimum is measured against, so a missing price
/// could unlock a code the basket had not earned.
#[test]
fn a_missing_or_negative_price_is_refused_rather_than_treated_as_free() {
    let cat = menu(&[
        ("noprice", json!({ "name": "?", "available": true })),
        ("negative", json!({ "name": "?", "price": -1, "available": true })),
        ("text", json!({ "name": "?", "price": "800", "available": true })),
    ]);
    let none = Vec::new();
    for id in ["noprice", "negative", "text"] {
        assert_eq!(
            price_basket(&cat, [want(id, &none, 1)]).unwrap_err(),
            Refusal::NoPrice(id.into()),
            "{id}"
        );
    }
}

/// A QUANTITY OUT OF RANGE WAS CLAMPED INTO ONE by the preview, so a basket
/// asking for zero of something was quoted for one.
#[test]
fn a_quantity_out_of_range_is_refused_and_not_clamped() {
    let cat = menu(&[("d", dish("Dish", 100))]);
    let none = Vec::new();
    for q in [0, -1, 100, i64::MAX] {
        let e = price_basket(&cat, [want("d", &none, q)]).unwrap_err();
        assert_eq!(e, Refusal::Quantity, "quantity {q}");
        assert_eq!(e.status(), 400);
    }
    assert!(price_basket(&cat, [want("d", &none, 99)]).is_ok(), "99 is inside the range");
}

/// AN UNKNOWN DISH FAILS CLOSED, and it is the only failure the old preview
/// had. The 400 says the browser's menu is stale.
#[test]
fn an_unknown_dish_is_refused_by_name() {
    let cat = menu(&[("d", dish("Dish", 100))]);
    let none = Vec::new();
    let e = price_basket(&cat, [want("ghost", &none, 1)]).unwrap_err();
    assert_eq!(e.text(), "unknown product: ghost");
    assert_eq!(e.status(), 400);
}

/// A CATALOGUE THIS VENUE WROTE AND CANNOT READ BACK IS THE PLATFORM'S FAULT.
/// Telling the customer to fix their basket would be a lie, so it is a 500.
#[test]
fn an_unreadable_record_is_the_platforms_fault_and_says_so() {
    let broken = |_: &str| Some("{not json".to_string());
    let none = Vec::new();
    let e = price_basket(broken, [want("d", &none, 1)]).unwrap_err();
    assert_eq!(e.status(), 500);
    assert!(e.text().starts_with("catalogue product unreadable: "), "{}", e.text());
}

/// A NEGATIVE OPTION DELTA MUST NOT MAKE A LINE PAY THE CUSTOMER — and the
/// floor is per unit, not on the subtotal, or one discounted line would eat
/// another line's money.
#[test]
fn a_negative_delta_floors_the_line_at_zero_and_not_the_basket() {
    let cat = menu(&[
        ("cheap", json!({
            "name": "Cheap", "price": 100, "available": true,
            "modifierGroups": [{ "id": "g", "name": "G", "min": 0, "max": 1,
                "options": [{ "id": "off", "name": "Off", "priceDelta": -500 }] }]
        })),
        ("full", dish("Full", 700)),
    ]);
    let off = vec!["off".to_string()];
    let none = Vec::new();
    let b = price_basket(&cat, [want("cheap", &off, 3), want("full", &none, 1)]).unwrap();
    assert_eq!(b.lines[0].unit_price, 0, "floored at zero, not -400");
    assert_eq!(b.subtotal, 700, "and it takes nothing off the other line");
}

/// THE FIRST REFUSAL IS THE ANSWER, and it is reported for the line that
/// caused it — a basket is not partly priced.
#[test]
fn one_bad_line_refuses_the_whole_basket() {
    let cat = menu(&[("good", dish("Good", 100))]);
    let none = Vec::new();
    let e = price_basket(&cat, [want("good", &none, 1), want("ghost", &none, 1)]).unwrap_err();
    assert_eq!(e, Refusal::Unknown("ghost".into()));
}

/// AN EMPTY BASKET COSTS NOTHING and is not an error here: whether a venue
/// accepts one is the minimum-order rule's business, one layer up.
#[test]
fn an_empty_basket_is_priced_at_zero() {
    let cat = menu(&[]);
    let b = price_basket(&cat, []).unwrap();
    assert_eq!(b.subtotal, 0);
    assert!(b.lines.is_empty());
}
