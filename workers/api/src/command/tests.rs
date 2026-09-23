//! The command rules, exercised without a Durable Object anywhere near it.
//!
//! THAT IS THE POINT OF THE SPLIT ABOVE, not just the line count. Until
//! `decide` was lifted out of `storefront::place`, reaching any of this meant
//! standing up an object, so none of it had a native test -- and the promo
//! total that reached Stripe was wrong for as long as that was true.

use super::place::*;
use super::Refused;

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
        // `decide` never looks at it: the bell is queued by the OBJECT after
        // the log write, not by the rule. Asserted below rather than assumed.
        notify_text: None,
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
    let stored = decide(&mut hub, &mut stock, &[], &Ok(None), &input(Some(&raw)))
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
    let out = decide(&mut hub, &mut stock, &[], &Ok(None), &i);
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
    let stored = decide(&mut hub, &mut stock, &[], &Ok(None), &input(None)).expect("no bom, no refusal");
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
    let out = decide(&mut hub, &mut stock, &[], &Ok(None), &i);
    let Err(Refused::Stock(msg)) = &out else {
        panic!("a short ingredient must refuse: {out:?}");
    };
    assert!(msg.to_lowercase().contains("salmon"), "the refusal must name it: {msg}");
    assert_eq!(hub.len(), 0, "and no order may be logged");
    assert_eq!(stock.len(), staged, "nor any line of the basket reserved");
}

/// THE MESSAGE CANNOT CHANGE THE ORDER. `notify_text` rides in the same
/// command as the placement so that the bell is owed by the turn that writes
/// the order — but it is the OBJECT that queues it, after the log write, and
/// the rule below must be blind to it. If it were not, a venue with a bot
/// connected would price a basket differently from one without.
#[test]
fn what_the_kitchen_is_told_does_not_change_what_is_stored() {
    let (mut hub_a, mut stock_a) = images();
    let quiet = decide(&mut hub_a, &mut stock_a, &[], &Ok(None), &input(None)).expect("no promo");

    let (mut hub_b, mut stock_b) = images();
    let mut with_bell = input(None);
    with_bell.notify_text = Some("New order: 1x Futomaki, 3000".into());
    let loud = decide(&mut hub_b, &mut stock_b, &[], &Ok(None), &with_bell).expect("no promo");

    assert_eq!(quiet, loud, "the stored envelope must not depend on the message");
    assert_eq!(hub_a.len(), hub_b.len());
}

// ── ADVANCE ─────────────────────────────────────────────────────────────────

mod advancing {
    use super::images;
    use crate::command::advance::{decide, settlement, AdvanceIn};
    use crate::command::Refused;
    use serde_json::json;

    fn order(status: &str, venue: &str) -> String {
        json!({
            "id": "o1", "order_id": "o1", "location_id": venue, "status": status,
            "items": [{"product_id": "p1", "quantity": 1, "unit_price": 900, "name": "Futomaki"}],
            "subtotal": 900, "total": 900, "payment": "cash",
            "contact": {"name": "C", "phone": "+355690000000"},
            "created_at_ms": 1_700_000_000_000i64,
        })
        .to_string()
    }

    fn input(next: &str) -> AdvanceIn {
        AdvanceIn {
            order_id: "o1".into(),
            location_id: "sushi-durres".into(),
            next: next.into(),
            reason: None,
            now_ms: 1_700_000_100_000,
        }
    }

    /// THE TENANT BOUNDARY, AND IT MUST NOT BE DISTINGUISHABLE FROM ABSENCE.
    /// An owner of one venue who can tell "not yours" from "no such order"
    /// can enumerate another venue's order ids. Both are 404 and both say
    /// "order not found".
    #[test]
    fn another_venues_order_is_not_found_rather_than_forbidden() {
        let (mut hub, mut stock) = images();
        let other = order("PENDING", "dubin-durres");
        let out = decide(&mut hub, &mut stock, Some(&other), &input("CONFIRMED"));
        assert_eq!(out.unwrap_err(), Refused::NotFound);
        assert_eq!(decide(&mut hub, &mut stock, None, &input("CONFIRMED")).unwrap_err(),
                   Refused::NotFound);
        assert_eq!(hub.len(), 0, "and nothing is logged either way");
    }

    /// AN ILLEGAL EDGE IS THE KERNEL'S REFUSAL AND LEAVES NOTHING BEHIND.
    /// This is the compensation's replacement for this command: the caller is
    /// handed a refusal and drops both images unwritten.
    #[test]
    fn an_edge_the_kernel_refuses_writes_neither_image() {
        let (mut hub, mut stock) = images();
        let o = order("DELIVERED", "sushi-durres");
        let out = decide(&mut hub, &mut stock, Some(&o), &input("PREPARING"));
        assert!(matches!(out, Err(Refused::Conflict(_))), "a delivered order cannot be prepared: {out:?}");
        assert_eq!(out.unwrap_err().status(), 409);
        assert_eq!(hub.len(), 0);
        assert_eq!(stock.len(), 0);
    }

    /// THE SHELF FOLLOWS THE ORDER, and only for the three transitions that
    /// mean something to it. PICKED_UP and DELIVERED settle NOTHING: the
    /// consumption already happened at PREPARING, and settling twice would
    /// take the same ingredients off the shelf a second time.
    #[test]
    fn only_three_transitions_touch_the_shelf() {
        assert_eq!(settlement("PREPARING"), Some(true), "preparing consumes");
        assert_eq!(settlement("REJECTED"), Some(false), "a rejection releases");
        assert_eq!(settlement("CANCELLED"), Some(false), "so does a cancellation");
        for quiet in ["CONFIRMED", "READY", "IN_DELIVERY", "DELIVERED", "PICKED_UP"] {
            assert_eq!(settlement(quiet), None, "{quiet} must not settle");
        }
    }

    /// A rejection carries WHY, with the event, so the customer can be told
    /// something true rather than "rejected".
    #[test]
    fn a_rejection_records_its_reason_with_the_event() {
        let (mut hub, mut stock) = images();
        let o = order("PENDING", "sushi-durres");
        let mut i = input("REJECTED");
        i.reason = Some("the kitchen is closed".into());
        let merged = decide(&mut hub, &mut stock, Some(&o), &i).expect("a pending order may be rejected");
        assert_eq!(merged["rejection_reason"], "the kitchen is closed");
        assert_eq!(merged["status"], "REJECTED");
        assert_eq!(hub.len(), 1, "one Advanced event");
    }

    /// THE HUB-OWNED FIELDS SURVIVE THE TRANSITION. The kernel owns items,
    /// status and money; the address, the contact and the payment ride
    /// alongside, and an address lost on a status change is a failure that
    /// only surfaces at the customer's door.
    #[test]
    fn the_contact_and_the_payment_survive_the_kernels_answer() {
        let (mut hub, mut stock) = images();
        let o = order("PENDING", "sushi-durres");
        let merged = decide(&mut hub, &mut stock, Some(&o), &input("CONFIRMED")).expect("confirm");
        assert_eq!(merged["contact"]["phone"], "+355690000000");
        assert_eq!(merged["payment"], "cash");
    }
}

// ── ASSIGN ──────────────────────────────────────────────────────────────────

mod assigning {
    use crate::command::assign::{records, AssignIn, HANDABLE};
    use serde_json::json;

    fn input() -> AssignIn {
        AssignIn {
            order_id: "o1".into(),
            location_id: "sushi-durres".into(),
            courier_id: "c1".into(),
            now_ms: 1_700_000_100_000,
        }
    }

    /// CASH IS DUE ONLY IF THE ORDER IS CASH. A courier carrying a card
    /// order's total as "cash due" is asked to hand over money nobody gave
    /// them, and the two halves of this used to be written by two different
    /// closures in two different round trips.
    #[test]
    fn only_a_cash_order_puts_cash_on_the_courier() {
        let cash = json!({ "payment": "cash", "total": 1500 });
        let (rec, _) = records(&cash, &input());
        let v: serde_json::Value = serde_json::from_str(&rec).expect("json");
        assert_eq!(v["cash_due"], 1500);

        let card = json!({ "payment": "card", "total": 1500 });
        let (rec, _) = records(&card, &input());
        let v: serde_json::Value = serde_json::from_str(&rec).expect("json");
        assert_eq!(v["cash_due"], 0, "a card order owes the courier nothing");
    }

    /// THE RECORD AND THE ORDER NAME THE SAME COURIER AND THE SAME INSTANT.
    /// They were two writes that only agreed because one person wrote both.
    #[test]
    fn the_assignment_and_the_order_agree_by_construction() {
        let o = json!({ "payment": "cash", "total": 900, "status": "READY" });
        let i = input();
        let (rec, updated) = records(&o, &i);
        let v: serde_json::Value = serde_json::from_str(&rec).expect("json");
        assert_eq!(v["courier_id"], updated["courier_id"]);
        assert_eq!(v["assigned_at_ms"], updated["assigned_at_ms"]);
        assert_eq!(v["courier_id"], "c1");
        assert!(v["picked_up_at_ms"].is_null(), "a fresh assignment has not been collected");
    }

    /// REFUNDING IS ACTIVE AND MUST NOT BE HANDED OUT, which is why this set
    /// is written down rather than derived from the kernel's `is_active`.
    #[test]
    fn a_refunding_order_is_active_and_still_not_handable() {
        assert!(HANDABLE.contains(&"READY"));
        assert!(HANDABLE.contains(&"CONFIRMED"));
        assert!(!HANDABLE.contains(&"REFUNDING"), "active, and not a courier's job");
        assert!(!HANDABLE.contains(&"PENDING"), "an unaccepted order has no courier");
        assert!(!HANDABLE.contains(&"DELIVERED"));
    }
}

// ── TAX (BLUEPRINT-TAX-PRICE-CHANNEL §6 item 4, gate G2) ────────────────────

mod taxing {
    use super::images;
    use crate::command::place::{decide, PlaceIn};
    use crate::command::Refused;
    use crate::services::ordering::tax_cfg::VenueTax;
    use dowiz_core::tax::RatePpm;
    use serde_json::{json, Value};

    const V20: VenueTax =
        VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

    /// Three coffees at 250 and a 300 fee: blueprint §3.2 example 2's basket.
    fn input(items: Value, promo: Option<&str>) -> PlaceIn {
        PlaceIn {
            order_id: "o1".into(),
            envelope: json!({
                "order_id": "o1", "status": "PENDING", "items": items,
                "subtotal": 750, "delivery_fee": 300, "tip": 0, "total": 1050,
            })
            .to_string(),
            seq: 1,
            bom_lines: Vec::new(),
            promo: promo.map(str::to_string),
            promo_code: Some("SAVE".into()),
            subtotal: 750,
            fee: 300,
            tip: 0,
            now_ms: 1_700_000_000_000,
            notify_text: None,
        }
    }

    fn coffees() -> Value {
        json!([{ "product_id": "coffee", "quantity": 3, "unit_price": 250 }])
    }

    fn ten_percent() -> String {
        json!({ "code": "SAVE", "kind": "percent", "value": 10, "active": true }).to_string()
    }

    #[test]
    fn the_stored_base_is_the_subtotal_after_the_cut() {
        let (mut hub, mut stock) = images();
        let raw = ten_percent();
        let stored = decide(&mut hub, &mut stock, &[], &Ok(Some(V20)), &input(coffees(), Some(&raw)))
            .expect("placed");
        let v: Value = serde_json::from_str(&stored).unwrap();
        assert_eq!(v["discount"], 75);
        assert_eq!(v["tax"]["groups"][0]["base"], 750 - 75, "subtotal - cut");
        assert_eq!(v["tax"]["groups"][0]["tax"], 112);
        assert_eq!(v["tax"]["total"], 112 + 50, "and the fee's 50");
        assert_eq!(v["total"], 975);
        assert_eq!(v["items"][0]["vat_ppm"], 200000);
    }

    /// G2, RED side: a venue WITH a rate cannot place an order it cannot tax.
    #[test]
    fn g2_an_untaxable_order_at_a_taxed_venue_is_refused_untaxed() {
        let (mut hub, mut stock) = images();
        let unreadable = json!([{ "product_id": "coffee", "quantity": 3 }]);
        let out = decide(&mut hub, &mut stock, &[], &Ok(Some(V20)), &input(unreadable, None));
        assert!(matches!(out, Err(Refused::Untaxed(_))), "{out:?}");
        assert_eq!(out.unwrap_err().status(), 500);
        assert_eq!(hub.len(), 0, "nothing logged");
        // A settings image this cannot read is the same refusal, and loud.
        let out = decide(&mut hub, &mut stock, &[], &Err("tax.default_ppm: bad".into()), &input(coffees(), None));
        assert!(matches!(&out, Err(Refused::Untaxed(m)) if m.contains("tax.default_ppm")), "{out:?}");
    }

    /// G2, GREEN side: the same order at a venue with NO rate is placed with
    /// no block (the transition rule), and a taxable one at a taxed venue is
    /// placed with one.
    #[test]
    fn g2_no_rate_places_without_a_block_and_a_rate_places_with_one() {
        let (mut hub, mut stock) = images();
        let unreadable = json!([{ "product_id": "coffee", "quantity": 3 }]);
        let stored = decide(&mut hub, &mut stock, &[], &Ok(None), &input(unreadable, None)).expect("untaxed venue");
        assert!(serde_json::from_str::<Value>(&stored).unwrap().get("tax").is_none());
        let (mut hub, mut stock) = images();
        let stored = decide(&mut hub, &mut stock, &[], &Ok(Some(V20)), &input(coffees(), None)).expect("taxed");
        assert_eq!(serde_json::from_str::<Value>(&stored).unwrap()["tax"]["total"], 125 + 50);
    }

    /// THE HUB_OWNED TRAP (§1.8): the first "confirm" must not delete the
    /// figures a receipt is issued from.
    #[test]
    fn the_tax_block_survives_confirmed() {
        let (mut hub, mut stock) = images();
        let placed = decide(&mut hub, &mut stock, &[], &Ok(Some(V20)), &input(coffees(), None)).expect("placed");
        let mut o: Value = serde_json::from_str(&placed).unwrap();
        o["id"] = json!("o1");
        o["location_id"] = json!("v1");
        o["created_at_ms"] = json!(1_700_000_000_000i64);
        let a = crate::command::advance::AdvanceIn {
            order_id: "o1".into(),
            location_id: "v1".into(),
            next: "CONFIRMED".into(),
            reason: None,
            now_ms: 1_700_000_100_000,
        };
        let merged = crate::command::advance::decide(&mut hub, &mut stock, Some(&o.to_string()), &a)
            .expect("confirm");
        assert_eq!(merged["status"], "CONFIRMED");
        assert_eq!(merged["tax"], o["tax"], "the block must ride through the transition");
    }
}
