//! The wiring rules, over an order built by the REAL kernel placement
//! (`json_api::place_order_at`) and taxed by the REAL stamp -- the envelope
//! `hubdo::place` actually holds when it calls `at_placement`.

use super::*;
use crate::fiscal::queue::{DEADLINE_MS, KIND};
use crate::services::ordering::tax_block::stamp;
use crate::services::ordering::tax_cfg::VenueTax;
use dowiz_core::tax::RatePpm;

const T: i64 = 1_790_000_000_000;
const V20: VenueTax = VenueTax { default: RatePpm(200_000), inclusive: true, fee: RatePpm(200_000) };

/// Two maki at 600 lek, placed at `at` through the kernel, cash at the door.
fn placed(id: &str, at: i64, taxed: bool) -> Value {
    let lines = json!([{ "product_id": "maki", "quantity": 2, "unit_price": 600 }]).to_string();
    let kernel = dowiz_kernel::json_api::place_order_at(id.into(), None, &lines, at, Some("storefront".into()))
        .expect("the kernel places it");
    let mut o: Value = serde_json::from_str(&kernel).unwrap();
    o["total"] = json!(1200);
    o["location_id"] = json!("v1");
    o["payment"] = json!("cash");
    if taxed {
        stamp(&mut o, &V20, 0, 0, 0).expect("the real stamp taxes it");
    }
    o
}

#[test]
fn the_setting_empty_is_off_an_integer_is_on_anything_else_is_loud() {
    assert_eq!(config(""), Config::Off);
    assert_eq!(config("  "), Config::Off);
    assert_eq!(config("1790000000000"), Config::From(T));
    assert!(matches!(config("2026-10-01"), Config::Invalid(w) if w.contains("epoch milliseconds")));
    assert!(matches!(config("-5"), Config::Invalid(_)));
}

#[test]
fn a_configured_venue_queues_one_fiscal_entry_for_a_placed_order() {
    let o = placed("o1", T + 10, true);
    let AtPlacement::Queued(e) = at_placement(&Config::From(T), &o, "ALL", T + 20) else {
        panic!("{:?}", at_placement(&Config::From(T), &o, "ALL", T + 20))
    };
    assert_eq!((e.kind.as_str(), e.to.as_str(), e.queued_at_ms), (KIND, "o1", T + 20));
    let h = health(&[e.clone()], T + 20);
    assert_eq!(h.first_deadline, Some(T + 20 + DEADLINE_MS));
    // Placing the same order again (an idempotent replay) is the same entry id.
    let AtPlacement::Queued(again) = at_placement(&Config::From(T), &o, "ALL", T + 99) else { panic!() };
    assert_eq!(again.id, e.id);
}

#[test]
fn an_unconfigured_venue_queues_nothing() {
    let o = placed("o1", T + 10, true);
    assert_eq!(at_placement(&Config::Off, &o, "ALL", T + 20), AtPlacement::NotConfigured);
    let bad = config("tomorrow");
    assert_eq!(at_placement(&bad, &o, "ALL", T + 20), AtPlacement::NotConfigured);
}

#[test]
fn an_order_placed_before_since_is_not_owed_and_one_at_since_is() {
    let early = placed("o0", T - 1, true);
    assert_eq!(at_placement(&Config::From(T), &early, "ALL", T + 5), AtPlacement::Before);
    let exact = placed("o1", T, true);
    assert!(matches!(at_placement(&Config::From(T), &exact, "ALL", T + 5), AtPlacement::Queued(_)));
}

#[test]
fn a_configured_venue_with_no_tax_block_is_a_refusal_not_silence() {
    let o = placed("o1", T + 10, false);
    assert_eq!(at_placement(&Config::From(T), &o, "ALL", T + 20), AtPlacement::Refused(Refusal::NoTax));
}

#[test]
fn an_order_from_the_platform_or_untrusted_is_not_owed() {
    let mut o = placed("o1", T + 10, true);
    o["external"] = json!({ "fic": "FIC-1" });
    assert_eq!(at_placement(&Config::From(T), &o, "ALL", T + 20), AtPlacement::NotOwed);
    let mut o = placed("o1", T + 10, true);
    o["price_trusted"] = json!(false);
    assert_eq!(at_placement(&Config::From(T), &o, "ALL", T + 20), AtPlacement::NotOwed);
}

#[test]
fn health_says_not_configured_and_says_a_bad_value_out_loud() {
    let off = health_json(&Config::Off, Ok(&[]), T);
    assert_eq!(off["configured"], json!(false));
    assert!(off["said"].as_str().unwrap().contains("not configured"));
    assert!(off.get("backlog").is_none(), "no numbers for a queue that does not exist");

    let bad = health_json(&config("2026-10-01"), Ok(&[]), T);
    assert_eq!(bad["configured"], json!(false));
    assert!(bad["error"].as_str().unwrap().contains("fiscal.since_ms"));
}

#[test]
fn health_of_a_configured_venue_names_backlog_deadline_and_the_overdue_order() {
    let AtPlacement::Queued(old) = at_placement(&Config::From(0), &placed("o-old", 5, true), "ALL", T) else {
        panic!()
    };
    let now = T + 49 * 3600 * 1000;
    let h = health_json(&Config::From(0), Ok(&[old]), now);
    assert_eq!(h["configured"], json!(true));
    assert_eq!(h["sender"], json!("not_configured"));
    assert_eq!(h["backlog"], json!(1));
    assert_eq!(h["oldest_issued_at"], json!(T));
    assert_eq!(h["first_deadline"], json!(T + DEADLINE_MS));
    assert_eq!(h["overdue"][0]["order_id"], json!("o-old"));

    let unreadable = health_json(&Config::From(0), Err("fiscal image is unreadable".into()), now);
    assert_eq!(unreadable["error"], json!("fiscal image is unreadable"));
    assert!(unreadable.get("backlog").is_none(), "an unreadable queue is not an empty one");
}
