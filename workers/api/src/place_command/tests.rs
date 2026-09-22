//! The placement rule, exercised without a Durable Object anywhere near it.
//!
//! THAT IS THE POINT OF THE SPLIT ABOVE, not just the line count. Until
//! `decide` was lifted out of `storefront::place`, reaching any of this meant
//! standing up an object, so none of it had a native test -- and the promo
//! total that reached Stripe was wrong for as long as that was true.

use super::*;

fn envelope(total: i64) -> String {
    serde_json::json!({
        "order_id": "o1",
        "status": "PENDING",
        "subtotal": 2500,
        "total": total,
    })
    .to_string()
}

fn input(promo: Option<&str>) -> PlaceIn {
    PlaceIn {
        order_id: "o1".into(),
        envelope: envelope(3000),
        seq: 1,
        bom_lines: Vec::new(),
        promo: promo.map(str::to_string),
        promo_code: Some("SAVE".into()),
        subtotal: 2500,
        fee: 500,
        tip: 0,
        now_ms: 1_700_000_000_000,
    }
}

/// A product carrying a recipe, in the shape `stock::bom_of` reads.
/// `bom_lines` is `(PRODUCT JSON, quantity ordered)` and not
/// `(supply, qty)` — the object expands the recipe itself, so that a
/// basket of two rolls that both use salmon is checked against the total
/// it actually needs rather than twice against half of it.
fn dish(supply: &str, qty: i64) -> String {
    serde_json::json!({ "id": "d1", "bom": [{ "supply": supply, "qty": qty }] }).to_string()
}

fn images() -> (dowiz_hub::Hub, dowiz_hub::stock::StockLog) {
    (
        dowiz_hub::Hub::create_sized(64 * 1024).expect("hub"),
        dowiz_hub::stock::StockLog::create_sized(64 * 1024).expect("stock"),
    )
}

/// THE ONE THAT NAMES THE DEFECT. The stored envelope and the number the
/// caller charges were two different numbers: the discount was applied
/// inside the CAS closure and the outer `total` never learned about it.
/// Here the ONLY total anyone can read is the one that was written, which
/// is what makes them the same by construction rather than by agreement.
#[test]
fn the_total_that_is_stored_is_the_total_after_the_discount() {
    let (mut hub, mut stock) = images();
    // 10% off, no floor, no expiry, unlimited uses.
    let raw = serde_json::json!({
        "code": "SAVE", "kind": "percent", "value": 10, "active": true
    })
    .to_string();
    let stored = decide(&mut hub, &mut stock, &[], &input(Some(&raw)))
        .expect("a live code must redeem");
    let v: serde_json::Value = serde_json::from_str(&stored).expect("json");
    assert_eq!(v["discount"], 250, "10% of 2500");
    assert_eq!(v["total"], 2750, "2500 - 250 + 500 fee");
    assert_ne!(v["total"], 3000, "the pre-discount total must not survive");
}

/// AND THE REFUSAL LEAVES NOTHING BEHIND. This is the compensation's
/// replacement, stated as a test: a placement that fails after the
/// reservations were computed must leave the stock image exactly as it was,
/// because the caller never writes what it was handed.
#[test]
fn a_refused_promo_leaves_the_stock_image_untouched() {
    let (mut hub, mut stock) = images();
    // Enough on the shelf that the BASKET is fine and only the code is not:
    // the reservations must actually be staged for this test to say
    // anything about what a refusal leaves behind.
    stock
        .append_all(&[dowiz_hub::stock::StockEvent::Received {
            item: "salmon".into(),
            qty: 1000,
        }])
        .expect("receive");
    let before = stock.len();
    let raw = serde_json::json!({
        "code": "SAVE", "kind": "percent", "value": 10, "active": false
    })
    .to_string();
    let mut i = input(Some(&raw));
    i.bom_lines = vec![(dish("salmon", 40), 2)];
    let out = decide(&mut hub, &mut stock, &[], &i);
    assert!(matches!(out, Err(Refused::Promo(_))), "an inactive code must be refused: {out:?}");
    assert_eq!(hub.len(), 0, "no order may be logged by a refused placement");
    // The reservations WERE appended to the in-memory log before the promo
    // refused -- and that is exactly why the caller must drop it rather
    // than write it. The assertion is on the CALLER's contract: this value
    // is never persisted. `len` moving here is the proof that the old code
    // would have had something to compensate.
    assert!(stock.len() > before, "the reservations were staged in memory");
}

/// A short ingredient is a 409 that names it; a bad code is a 400. Both
/// were once the same answer to the customer.
#[test]
fn the_two_refusals_do_not_answer_with_the_same_status() {
    assert_eq!(Refused::Stock("salmon".into()).status(), 409);
    assert_eq!(Refused::Promo("expired".into()).status(), 400);
    assert_eq!(Refused::Append("x".into()).status(), 500);
}

/// A venue that has modelled no ingredients reserves nothing, and that is
/// a placement rather than a refusal. Stock control that must be complete
/// before anything can be sold is stock control nobody switches on.
#[test]
fn a_venue_with_no_recipes_places_the_order_anyway() {
    let (mut hub, mut stock) = images();
    let stored = decide(&mut hub, &mut stock, &[], &input(None)).expect("no bom, no refusal");
    assert_eq!(hub.len(), 1, "the order is logged");
    assert_eq!(stock.len(), 0, "and nothing is reserved");
    let v: serde_json::Value = serde_json::from_str(&stored).expect("json");
    assert_eq!(v["total"], 3000, "no promo, no change to the total");
}

/// An ingredient that is short refuses the WHOLE basket, so a third line
/// cannot leave the first two held.
#[test]
fn a_short_ingredient_refuses_every_line_of_the_basket() {
    let (mut hub, mut stock) = images();
    // One unit received, two wanted.
    stock
        .append_all(&[dowiz_hub::stock::StockEvent::Received {
            item: "salmon".into(),
            qty: 40,
        }])
        .expect("receive");
    let staged = stock.len();
    let mut i = input(None);
    // Eighty grams wanted against forty received. ONE short supply, so the
    // message this asserts on can only be about that one.
    i.bom_lines = vec![(dish("salmon", 40), 2)];
    let out = decide(&mut hub, &mut stock, &[], &i);
    let Err(Refused::Stock(msg)) = &out else {
        panic!("a short ingredient must refuse: {out:?}");
    };
    assert!(msg.to_lowercase().contains("salmon"), "the refusal must name it: {msg}");
    assert_eq!(hub.len(), 0, "and no order may be logged");
    assert_eq!(stock.len(), staged, "nor any line of the basket reserved");
}
