//! The welcome offer over synthetic logs, through the real `command::place::decide`
//! and the real promo engine. Every refusal has its positive twin.

use super::*;
use crate::command::place::{decide, PlaceIn};

const VENUE: &str = "loc_1";
const PHONE: &str = "+355691234567";
const NOW: i64 = 1_790_000_000_000;

fn welcome(offer: Offer) -> WelcomeIn {
    WelcomeIn { offer, location_id: VENUE.into(), phones: vec![PHONE.into()], campaign: Some("spring".into()) }
}

fn fixed() -> Offer {
    Offer::Fixed { value: 300, min: 2000 }
}

fn envelope(id: &str, phone: &str) -> Value {
    json!({
        "id": id, "location_id": VENUE, "status": "PENDING", "subtotal": 3000, "total": 3500,
        "created_at_ms": NOW, "contact": { "phone": phone }, "source": "storefront",
        "items": [
            { "product_id": "roll", "quantity": 2, "unit_price": 1200 },
            { "product_id": "edamame", "quantity": 1, "unit_price": 600 }
        ],
    })
}

fn place_in(id: &str, phone: &str, w: Option<WelcomeIn>) -> PlaceIn {
    PlaceIn {
        order_id: id.into(), envelope: envelope(id, phone).to_string(), seq: u64::from(id.as_bytes()[0]),
        bom_lines: Vec::new(), promo: None, promo_code: None, subtotal: 3000, fee: 500, tip: 0, now_ms: NOW,
        notify_text: None, stamps: None, welcome: w,
    }
}

fn images() -> (dowiz_hub::Hub, dowiz_hub::stock::StockLog) {
    (dowiz_hub::Hub::create_sized(64 * 1024).unwrap(), dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap())
}

fn view(id: &str, o: Value) -> OrderView {
    OrderView { order_id: id.into(), kind: 1, seq: 0, order_json: o.to_string() }
}

fn granted(status: &str, phone: &str) -> OrderView {
    let mut o = envelope("old", phone);
    o["status"] = json!(status);
    o[RECORD] = json!({ "kind": "fixed", "discount": 300, "c": null });
    view("old", o)
}

/// RED PROOF 6.1: THE SAME PHONE CANNOT TAKE IT TWICE. The first placement is
/// granted; the second, decided over the log the first appended to, is placed
/// at full price and says `used`.
#[test]
fn a_second_use_by_the_same_phone_is_refused_politely() {
    let (mut hub, mut stock) = images();
    let first = decide(&mut hub, &mut stock, &[], &Ok(None), &place_in("a", PHONE, Some(welcome(fixed())))).unwrap();
    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!(first[RECORD], json!({ "kind": "fixed", "discount": 300, "c": "spring" }));
    assert_eq!((first["discount"].as_i64(), first["total"].as_i64()), (Some(300), Some(3200)));
    let after: Vec<OrderView> = crate::hubstore::orders_state(&hub).into_iter().map(OrderView::of_event).collect();
    let second = decide(&mut hub, &mut stock, &after, &Ok(None), &place_in("b", PHONE, Some(welcome(fixed())))).unwrap();
    let second: Value = serde_json::from_str(&second).unwrap();
    assert!(second.get(RECORD).is_none(), "{second}");
    assert_eq!(second[REFUSED], "used");
    assert_eq!(second["total"], 3500, "placed, at full price -- the order is not refused, the bonus is");
}

/// TWIN: another phone on the same log is still welcome.
#[test]
fn another_phone_is_still_welcome() {
    let log = vec![granted("DELIVERED", "+355690000009")];
    let mut env = envelope("b", PHONE);
    apply(&mut env, &log, &welcome(fixed()), 3000, 500, 0, NOW);
    assert_eq!(env["discount"], 300);
    assert!(env.get(REFUSED).is_none());
}

/// A venue that refused the first order gives the welcome back.
#[test]
fn a_refused_order_gives_the_welcome_back() {
    let mut env = envelope("b", PHONE);
    apply(&mut env, &[granted("REJECTED", PHONE)], &welcome(fixed()), 3000, 500, 0, NOW);
    assert_eq!(env["discount"], 300, "{env}");
    let mut env = envelope("c", PHONE);
    apply(&mut env, &[granted("DELIVERED", PHONE)], &welcome(fixed()), 3000, 500, 0, NOW);
    assert_eq!(env[REFUSED], "used");
}

/// RED PROOF 6.3: THE BONUS NEVER CHANGES A MENU PRICE. Every line's unit
/// price and the subtotal are what the pricer said; only `discount`/`total` move.
#[test]
fn the_bonus_never_changes_a_menu_price() {
    for offer in [fixed(), Offer::Gift { product: "edamame".into() }, Offer::Stamps] {
        let mut env = envelope("a", PHONE);
        let before = env["items"].clone();
        apply(&mut env, &[], &welcome(offer.clone()), 3000, 500, 0, NOW);
        assert_eq!(env["items"], before, "{offer:?}: the lines are untouched");
        assert_eq!(env["subtotal"], 3000, "{offer:?}");
        let cut = env["discount"].as_i64().unwrap_or(0);
        assert_eq!(env["total"].as_i64().unwrap_or(3500), 3000 - cut + 500, "{offer:?}");
    }
}

/// Below the minimum: refused by word, nothing taken off. TWIN at the minimum.
#[test]
fn a_basket_below_the_minimum_gets_nothing() {
    let w = welcome(Offer::Fixed { value: 300, min: 3001 });
    let mut env = envelope("a", PHONE);
    apply(&mut env, &[], &w, 3000, 500, 0, NOW);
    assert_eq!((env[REFUSED].as_str(), env.get("discount")), (Some("below_minimum"), None));
    let w = welcome(Offer::Fixed { value: 300, min: 3000 });
    let mut env = envelope("a", PHONE);
    apply(&mut env, &[], &w, 3000, 500, 0, NOW);
    assert_eq!(env["discount"], 300);
}

/// The gift is the dish's own price, once, only when it is in the basket.
#[test]
fn the_gift_is_one_of_that_dish_when_it_is_in_the_basket() {
    let mut env = envelope("a", PHONE);
    apply(&mut env, &[], &welcome(Offer::Gift { product: "edamame".into() }), 3000, 500, 0, NOW);
    assert_eq!((env["discount"].as_i64(), env["total"].as_i64()), (Some(600), Some(2900)));
    let mut env = envelope("a", PHONE);
    apply(&mut env, &[], &welcome(Offer::Gift { product: "gyoza".into() }), 3000, 500, 0, NOW);
    assert_eq!(env[REFUSED], "gift_missing");
    assert!(env.get(RECORD).is_none());
}

/// Stacks AFTER a code: never more than what is left.
#[test]
fn it_never_takes_more_than_is_left() {
    let mut env = envelope("a", PHONE);
    env["discount"] = json!(2900);
    apply(&mut env, &[], &welcome(fixed()), 3000, 500, 0, NOW);
    assert_eq!((env["discount"].as_i64(), env["total"].as_i64()), (Some(3000), Some(500)));
    let mut env = envelope("a", PHONE);
    env["discount"] = json!(3000);
    apply(&mut env, &[], &welcome(fixed()), 3000, 500, 0, NOW);
    assert_eq!(env[REFUSED], "nothing_left");
}

/// A double stamp takes nothing off and is recorded; the stamp fold counts it twice.
#[test]
fn a_double_stamp_is_recorded_and_counts_two() {
    let mut env = envelope("a", PHONE);
    apply(&mut env, &[], &welcome(Offer::Stamps), 3000, 500, 0, NOW);
    assert_eq!(env[RECORD]["kind"], "stamps");
    assert!(env.get("discount").is_none());
    env["status"] = json!("DELIVERED");
    assert_eq!(stamp_weight(&env), 2);
    let log = vec![view("a", env)];
    let n = crate::services::loyalty::stamps::count(&log, |_| true);
    assert_eq!(n, 2, "one delivered order with the double stamp");
    assert_eq!(stamp_weight(&envelope("b", PHONE)), 1);
}

/// The owner's form: each refusal in words, and its twin.
#[test]
fn the_form_refuses_what_gives_money_away_by_mistake() {
    let f = |kind: &str, value: Option<i64>, min: Option<i64>, product: Option<&str>| OfferIn {
        kind: kind.into(), value, min, product: product.map(str::to_string),
    };
    assert_eq!(check(&f("fixed", Some(0), None, None), false), Err("bag_value: a whole amount above 0"));
    assert_eq!(check(&f("fixed", Some(MAX_VALUE + 1), None, None), false).is_err(), true);
    assert_eq!(check(&f("fixed", Some(300), Some(-1), None), false).is_err(), true);
    assert_eq!(check(&f("fixed", Some(300), Some(2000), None), false), Ok(Some(fixed())));
    assert!(check(&f("gift", None, None, Some(" ")), false).is_err());
    assert_eq!(check(&f("gift", None, None, Some("edamame")), false), Ok(Some(Offer::Gift { product: "edamame".into() })));
    assert!(check(&f("stamps", None, None, None), false).is_err(), "no card, no double stamp");
    assert_eq!(check(&f("stamps", None, None, None), true), Ok(Some(Offer::Stamps)));
    assert_eq!(check(&f("off", None, None, None), true), Ok(None));
    assert!(check(&f("percent", Some(10), None, None), true).is_err());
    assert!(serde_json::from_value::<OfferIn>(json!({"kind": "fixed", "value": 3, "until": 1})).is_err(), "unknown field");
    assert_eq!(check_pct(""), Ok(None));
    assert_eq!(check_pct("25"), Ok(Some(25)));
    assert!(check_pct("0").is_err() && check_pct("61").is_err() && check_pct("22.5").is_err());
    for o in [fixed(), Offer::Gift { product: "x".into() }, Offer::Stamps] {
        assert_eq!(Offer::parse(&o.to_json().to_string()), Some(o.clone()), "round trip");
    }
    assert_eq!(Offer::parse(r#"{"kind":"fixed","value":0}"#), None, "a stored bad value gives nothing");
}
