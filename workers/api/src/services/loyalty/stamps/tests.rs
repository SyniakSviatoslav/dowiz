//! The stamp card, over synthetic logs. Every refusal has its positive twin,
//! and every test calls the real fold, the real resolver, the real promo
//! engine and the real `command::place::decide`.

use super::*;
use crate::command::place::{decide, PlaceIn};
use crate::services::customers::alias::{rule_link, Aliases};
use crate::services::customers::handlers::customer_key;
use crate::services::customers::identity::alias_at_placement;

const VENUE: &str = "loc_1";
const PHONE: &str = "+355691234567";
const NOW: i64 = 1_790_000_000_000;

fn view(id: &str, o: Value) -> OrderView {
    OrderView { order_id: id.into(), kind: 1, seq: 0, order_json: o.to_string() }
}

/// One order, `i` minutes into the day, under `phone`, in `status`.
fn order(i: i64, phone: &str, status: &str) -> OrderView {
    view(&format!("o{i}"), json!({
        "id": format!("o{i}"), "location_id": VENUE, "status": status, "total": 1000,
        "created_at_ms": NOW + i * 60_000, "contact": { "phone": phone },
    }))
}

fn delivered(n: i64) -> Vec<OrderView> {
    (0..n).map(|i| order(i, PHONE, if i % 2 == 0 { "DELIVERED" } else { "PICKED_UP" })).collect()
}

fn spent_card(i: i64, status: &str) -> OrderView {
    let mut v: Value = serde_json::from_str(&order(i, PHONE, status).order_json).unwrap();
    v["loyalty"] = json!({ "n": 10, "discount": 500 });
    view(&format!("o{i}"), v)
}

fn phones() -> Vec<String> {
    vec![PHONE.to_string()]
}

fn stamp_in() -> StampIn {
    StampIn { n: 10, reward_minor: 500, location_id: VENUE.into(), phones: phones() }
}

fn fold(listed: &[OrderView]) -> (i64, i64) {
    let p = phones();
    stamps(listed, by_phones(VENUE, &p), 10)
}

/// REFUSAL: cancelled, rejected, refunding and refunded orders, another
/// person's and another venue's are not stamps. TWIN: delivered and
/// collected ones are.
#[test]
fn only_what_they_had_is_a_stamp() {
    let mut log = delivered(3);
    for (i, st) in ["CANCELLED", "REJECTED", "REFUNDING", "COMPENSATED_REFUND", "PENDING", "READY"].iter().enumerate() {
        log.push(order(10 + i as i64, PHONE, st));
    }
    log.push(order(20, "+355697654321", "DELIVERED"));
    let mut elsewhere: Value = serde_json::from_str(&order(21, PHONE, "DELIVERED").order_json).unwrap();
    elsewhere["location_id"] = json!("loc_2");
    log.push(view("o21", elsewhere));
    assert_eq!(fold(&log), (3, 0));
}

/// THE CYCLE: the N-th order fills the card, the next placement spends it
/// once, the fold restarts, and the order that spent it is stamp 1.
#[test]
fn the_nth_fills_the_next_spends_and_the_fold_restarts() {
    let mut log = delivered(9);
    assert_eq!(fold(&log), (9, 0));
    assert_eq!(redeem(&log, &stamp_in(), 3000, NOW), None, "nine stamps spend nothing");
    log.push(order(9, PHONE, "DELIVERED"));
    assert_eq!(fold(&log), (0, 1));
    assert_eq!(redeem(&log, &stamp_in(), 3000, NOW), Some(500), "the tenth unlocks the reward");
    log.push(spent_card(10, "PENDING"));
    assert_eq!(fold(&log), (0, 0), "a spent card restarts the fold");
    assert_eq!(redeem(&log, &stamp_in(), 3000, NOW), None, "once per N stamps");
    log.push(spent_card(10, "DELIVERED"));
    log.remove(10);
    assert_eq!(fold(&log), (1, 0), "the order that spent it is the first stamp of the next card");
}

/// REFUSAL: a spent card on an order the venue refused is given back. TWIN:
/// on an order that took money it stays spent.
#[test]
fn a_refused_order_gives_the_card_back() {
    let mut log = delivered(10);
    log.push(spent_card(10, "REJECTED"));
    assert_eq!(fold(&log), (0, 1));
    log.pop();
    log.push(spent_card(10, "CONFIRMED"));
    assert_eq!(fold(&log), (0, 0));
}

/// A table's rounds: one stamp for the sitting once it is closed and PAID,
/// however many rounds; none while it is unpaid or still cooking.
#[test]
fn a_paid_sitting_is_one_stamp_and_an_unpaid_one_is_none() {
    let round = |i: i64, status: &str, paid: i64| {
        view(&format!("r{i}"), json!({
            "id": format!("r{i}"), "location_id": VENUE, "status": status, "total": 1000,
            "created_at_ms": NOW + i, "sitting_id": "s1", "placed_by": "guest",
            "contact": { "phone": PHONE }, "payments": [{ "amount": paid }],
        }))
    };
    let paid = vec![round(1, "DELIVERED", 1000), round(2, "DELIVERED", 1000), round(3, "DELIVERED", 1000)];
    assert_eq!(fold(&paid), (1, 0), "three rounds, one sitting, one stamp");
    let unpaid = vec![round(1, "DELIVERED", 1000), round(2, "DELIVERED", 0)];
    assert_eq!(fold(&unpaid), (0, 0), "a bill not settled is not a stamp");
    let cooking = vec![round(1, "DELIVERED", 1000), round(2, "PREPARING", 1000)];
    assert_eq!(fold(&cooking), (0, 0), "a table still eating is not a stamp");
}

/// TWO SPELLINGS OF ONE NUMBER ARE ONE CARD, through the C4 resolver: the
/// national spelling's first order writes the rule's link, and the
/// Worker's spellings cover both. TWIN: without the link, two cards.
#[test]
fn a_linked_alias_pair_is_one_card() {
    let secret = b"test-secret";
    let national = "0691234567";
    let (from, to) = (customer_key(secret, national), customer_key(secret, PHONE));
    assert_eq!(alias_at_placement(secret, national).as_deref(), Some(to.as_str()));
    let mut log: Vec<OrderView> = (0..6).map(|i| order(i, PHONE, "DELIVERED")).collect();
    log.extend((6..10).map(|i| order(i, national, "DELIVERED")));
    let card = |t: &dowiz_hub::table::Table| {
        let circle = Aliases::of(t).circle(&from, None);
        let p = spellings(&log, VENUE, |p| circle.contains(&customer_key(secret, p)), national);
        stamps(&log, by_phones(VENUE, &p), 10)
    };
    let mut people = dowiz_hub::table::Table::create(64 * 1024).unwrap();
    assert_eq!(card(&people), (4, 0), "unlinked: the national spelling's card alone");
    assert!(rule_link(&mut people, &from, &to, NOW));
    assert_eq!(card(&people), (0, 1), "linked: ten stamps on one card");
}

/// REFUSAL: a reward never exceeds what is left of the food. TWIN: beside a
/// promo code it stacks, and the total is the subtotal less both.
#[test]
fn the_reward_is_bounded_and_stacks_with_a_code() {
    let log = delivered(10);
    let big = StampIn { reward_minor: 5000, ..stamp_in() };
    assert_eq!(redeem(&log, &big, 2000, NOW), Some(2000));
    assert_eq!(redeem(&log, &big, 0, NOW), None, "nothing left, nothing spent");
    let mut env = json!({ "discount": 300, "total": 2700 });
    apply(&mut env, &log, &stamp_in(), 2500, 500, 0, NOW);
    assert_eq!(env["discount"], 800);
    assert_eq!(env["total"], 2500 - 300 - 500 + 500);
    assert_eq!(env["loyalty"], json!({ "n": 10, "discount": 500 }));
}

fn place_in(id: &str, stamps: Option<StampIn>) -> PlaceIn {
    let env = json!({
        "id": id, "location_id": VENUE, "status": "PENDING", "subtotal": 3000, "total": 3500,
        "created_at_ms": NOW + 3_600_000, "contact": { "phone": PHONE }, "source": "storefront",
    });
    PlaceIn {
        order_id: id.into(), envelope: env.to_string(), seq: u64::from(id.as_bytes()[0]), bom_lines: Vec::new(),
        promo: None, promo_code: None, subtotal: 3000, fee: 500, tip: 0, now_ms: NOW,
        notify_text: None, stamps,
    }
}

fn images() -> (dowiz_hub::Hub, dowiz_hub::stock::StockLog) {
    (dowiz_hub::Hub::create_sized(64 * 1024).unwrap(), dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap())
}

/// TWO PLACEMENTS RACE FOR ONE CARD: the object serialises them, and the
/// second counts over the log the first appended to -- it is priced without.
#[test]
fn two_placements_cannot_both_spend_one_card() {
    let (mut hub, mut stock) = images();
    let before = delivered(10);
    let first = decide(&mut hub, &mut stock, &before, &Ok(None), &place_in("a", Some(stamp_in()))).expect("first");
    let first: Value = serde_json::from_str(&first).unwrap();
    assert_eq!((first["discount"].as_i64(), first["total"].as_i64()), (Some(500), Some(3000)));
    let mut after = before.clone();
    after.extend(crate::hubstore::orders_state(&hub).into_iter().map(OrderView::of_event));
    let second = decide(&mut hub, &mut stock, &after, &Ok(None), &place_in("b", Some(stamp_in()))).expect("second");
    let second: Value = serde_json::from_str(&second).unwrap();
    assert!(second.get("loyalty").is_none(), "the second is priced without the card: {second}");
    assert_eq!(second["total"], 3500);
}

/// REFUSAL: a venue with the card off applies nothing. TWIN: on, it does.
#[test]
fn a_venue_without_a_card_applies_nothing() {
    let known = |on: &'static str, reward: &'static str| {
        move |k: &str| match k {
            ENABLED => on.to_string(),
            REWARD => reward.to_string(),
            _ => dowiz_hub::settings::KNOWN.iter().find(|x| x.key == k).map_or(String::new(), |x| x.default.to_string()),
        }
    };
    assert_eq!(config(known("0", "500")), None, "off by default and when switched off");
    assert_eq!(config(known("1", "")), None, "on, with no reward, gives nothing");
    assert_eq!(config(known("1", "500")), Some(Card { n: 10, reward_minor: 500 }), "the default card is ten");
    let (mut hub, mut stock) = images();
    let off = decide(&mut hub, &mut stock, &delivered(10), &Ok(None), &place_in("a", None)).unwrap();
    assert!(!off.contains("loyalty"), "{off}");
}

/// The three keys are declared, off by default, ten stamps by default.
#[test]
fn the_three_keys_are_declared_with_their_defaults() {
    let default = |k: &str| dowiz_hub::settings::KNOWN.iter().find(|x| x.key == k).map(|x| x.default);
    assert_eq!(default(ENABLED), Some("0"));
    assert_eq!(default(STAMPS_N), Some("10"));
    assert_eq!(default(REWARD), Some(""));
}

/// REFUSALS: out-of-range values are refused with the key named. TWINS: the
/// bounds themselves and a cleared value are accepted.
#[test]
fn settings_refuse_out_of_range_and_accept_the_bounds() {
    for (k, bad) in [(ENABLED, "2"), (ENABLED, "yes"), (STAMPS_N, "1"), (STAMPS_N, "21"), (STAMPS_N, "x"),
                     (REWARD, "0"), (REWARD, "-5"), (REWARD, "1.5")] {
        assert!(validate(k, bad).unwrap_err().starts_with(k), "{k}={bad} accepted");
    }
    for (k, good) in [(ENABLED, "0"), (ENABLED, "1"), (STAMPS_N, "2"), (STAMPS_N, "20"), (REWARD, "1"), (REWARD, ""),
                      ("tax.default_ppm", "anything")] {
        assert_eq!(validate(k, good), Ok(()), "{k}={good} refused");
    }
}

/// THE ORDER PAGE: the card starts at 1 with the order just placed (true,
/// it is on its way), a full card reads n / n, and a refused order adds
/// nothing.
#[test]
fn the_page_counts_this_order_honestly() {
    let p = phones();
    let this: Value = serde_json::from_str(&order(50, PHONE, "PENDING").order_json).unwrap();
    assert_eq!(shown(&[], by_phones(VENUE, &p), 10, &this), 1, "a first order is 1 / 10");
    let mut log = delivered(3);
    log.push(view("o50", this.clone()));
    assert_eq!(shown(&log, by_phones(VENUE, &p), 10, &this), 4);
    let full = delivered(10);
    let last: Value = serde_json::from_str(&full[9].order_json).unwrap();
    assert_eq!(shown(&full, by_phones(VENUE, &p), 10, &last), 10, "a full card reads 10 / 10");
    let refused: Value = serde_json::from_str(&order(51, PHONE, "REJECTED").order_json).unwrap();
    assert_eq!(shown(&delivered(3), by_phones(VENUE, &p), 10, &refused), 3);
}
