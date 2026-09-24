//! §2.9 by hand: each refusal beside its twin, the placement through the REAL
//! `place::decide`, and §2.9's CHECK — a marketplace order's total is never
//! recomputed by dowiz — through the kernel's own transitions.

use super::*;
use crate::command::place::{self, PlaceIn};
use dowiz_hub::stock::{StockEvent, StockLog};

const NOW: i64 = 1_790_000_000_000;

fn entry(total: i64) -> Entry {
    Entry {
        channel: channel::WOLT.into(),
        external_id: "W-8841".into(),
        lines: vec![
            Line { product_id: "maki".into(), quantity: 2, unit_price: 700 },
            Line { product_id: "beer".into(), quantity: 1, unit_price: 350 },
        ],
        discount: 150,
        total,
    }
}

fn built(e: &Entry) -> Result<(String, Value, i64), Refused> {
    envelope(e, "v1", "ALL", &[("maki".into(), "Maki".into())], NOW)
}

#[test]
fn a_wolt_order_is_the_platforms_priced_confirmed_pickup() {
    let (id, o, sub) = built(&entry(1600)).expect("adds up");
    assert_eq!(id, "wolt-W-8841");
    // PATH-SAFE AS IT STANDS: nothing `encodeURIComponent` would rewrite,
    // because the router does not decode what it rewrote (the 404 of 2026-09-24).
    assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'), "{id}");
    assert_eq!(sub, 1750);
    assert_eq!((o["status"].clone(), o["total"].clone(), o["discount"].clone()), (json!("CONFIRMED"), json!(1600), json!(150)));
    assert_eq!(o["price_trusted"], json!(false));
    assert_eq!(o["channel"], json!("wolt"));
    assert_eq!(o["payment"], json!("platform"), "no cash for a drawer or a courier");
    assert_eq!(o["fulfilment"]["kind"], json!("pickup"), "the platform's courier collects it");
    assert_eq!(o["external"], json!({ "source": "wolt", "order_id": "W-8841" }));
    assert_eq!(o["items"][0]["name"], json!("Maki"));
    assert!(o["at"]["CONFIRMED"].is_i64());
    // Law 3 on the stored shape: lines + fee + tip - discount = total.
    let lines: i64 = o["items"].as_array().unwrap().iter()
        .map(|i| i["unit_price"].as_i64().unwrap() * i["quantity"].as_i64().unwrap()).sum();
    assert_eq!(lines - o["discount"].as_i64().unwrap(), o["total"].as_i64().unwrap());
}

#[test]
fn a_total_that_is_not_lines_less_discount_is_refused() {
    // The commission netted into the total (22 % off 1600): a law-3 lie, refused.
    for t in [1248, 1750, 1601, -1] {
        assert!(matches!(built(&entry(t)), Err(Refused::Invalid(_))), "{t}");
    }
    assert!(built(&entry(1600)).is_ok(), "twin: the platform's total as charged");
    let mut e = entry(1600);
    e.discount = 1800;
    assert!(matches!(built(&e), Err(Refused::Invalid(_))), "a discount past the lines");
}

#[test]
fn only_a_marketplace_with_a_plain_id_is_entered_here() {
    for c in [channel::STOREFRONT, channel::CONSOLE, channel::EBILLS, "uber_eats"] {
        assert!(matches!(order_id(c, "W-1"), Err(Refused::Invalid(_))), "{c}");
    }
    for x in ["", "  ", "a b", "x/y", &"9".repeat(EXTERNAL_MAX + 1)] {
        assert!(matches!(order_id(channel::GLOVO, x), Err(Refused::Invalid(_))), "{x:?}");
    }
    assert_eq!(order_id(channel::BABOON, " B_7-x ").unwrap(), "baboon-B_7-x");
    assert_eq!(order_id(channel::GLOVO, &"9".repeat(EXTERNAL_MAX)).unwrap().len(), 6 + EXTERNAL_MAX);
}

#[test]
fn a_line_must_be_a_line() {
    let mut e = entry(0);
    e.lines.clear();
    assert!(built(&e).is_err());
    for (q, p) in [(0, 700), (QTY_MAX + 1, 700), (1, -1)] {
        let mut e = entry(700 * q - 150);
        e.lines = vec![Line { product_id: "maki".into(), quantity: q, unit_price: p }];
        assert!(matches!(built(&e), Err(Refused::Invalid(_))), "{q} {p}");
    }
}

#[test]
fn entering_it_twice_finds_the_first() {
    let listed = vec![OrderView { order_id: "wolt-W-8841".into(), kind: 1, seq: 1, order_json: "{}".into() }];
    assert!(existing(&listed, "wolt-W-8841").is_some());
    assert!(existing(&listed, "wolt-W-8842").is_none(), "twin: another platform order is new");
    assert!(existing(&listed, "glovo-W-8841").is_none(), "the same number on another platform is another order");
}

fn shelf(rice: i64) -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append(&StockEvent::Received { item: "rice".into(), qty: rice }).unwrap();
    s
}

fn place_in(o: &Value, sub: i64) -> PlaceIn {
    let maki = json!({ "id": "maki", "bom": [{ "supply": "rice", "qty": 100 }] }).to_string();
    PlaceIn {
        order_id: "wolt-W-8841".into(), envelope: o.to_string(), seq: NOW as u64,
        bom_lines: vec![(maki, 2)], promo: None, promo_code: None,
        subtotal: sub, fee: 0, tip: 0, now_ms: NOW, notify_text: None,
    }
}

#[test]
fn it_reserves_on_the_same_ledger_and_the_kernel_never_reprices_it() {
    let (_, o, sub) = built(&entry(1600)).unwrap();
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let mut s = shelf(1000);
    let stored = place::decide(&mut hub, &mut s, &[], &Ok(None), &place_in(&o, sub)).expect("placed");
    assert_eq!(s.ledger().unwrap().level("rice").reserved, 200);
    // §2.9 CHECK: through the kernel's own edges, the platform's total stays.
    let mut cur = stored;
    for next in ["PREPARING", "READY", "PICKED_UP"] {
        let up = dowiz_kernel::json_api::apply_event_logic(&cur, next).expect("a legal edge");
        let mut merged: Value = serde_json::from_str(&up).unwrap();
        let old: Value = serde_json::from_str(&cur).unwrap();
        crate::hubstore::carry_over(&old, &mut merged);
        assert_eq!((merged["total"].clone(), merged["price_trusted"].clone()), (json!(1600), json!(false)), "{next}");
        cur = merged.to_string();
    }
}

#[test]
fn an_out_of_stock_platform_order_is_refused_and_nothing_is_written() {
    let (_, o, sub) = built(&entry(1600)).unwrap();
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let mut s = shelf(150);
    let before = (hub.len(), s.to_bytes());
    assert!(matches!(place::decide(&mut hub, &mut s, &[], &Ok(None), &place_in(&o, sub)), Err(Refused::Stock(_))));
    assert_eq!((hub.len(), s.to_bytes()), before);
}

#[test]
fn the_bell_names_the_platform_its_number_and_the_dishes() {
    let e = entry(1600);
    let (_, o, _) = built(&e).unwrap();
    let text = bell(&o, &e, &[("maki".into(), "Maki".into())], "ALL", "Dubin");
    assert!(text.starts_with("WOLT W-8841\n"), "{text}");
    assert!(text.contains("pickup"), "the platform's courier collects it: {text}");
    assert!(text.contains("Maki"), "{text}");
    assert!(text.contains("beer"), "an unnamed dish falls back to its id: {text}");
    // Twin: another platform, another first line.
    let mut g = entry(1600);
    g.channel = channel::GLOVO.into();
    let (_, og, _) = built(&g).unwrap();
    assert!(bell(&og, &g, &[], "ALL", "Dubin").starts_with("GLOVO W-8841\n"));
}

/// THE FORM CANNOT CLAIM A FIRST-PARTY SOURCE: a body that says `console`,
/// `storefront` or `ebills` builds no envelope at all, so no Placed carries it.
#[test]
fn an_entry_that_names_a_first_party_source_builds_nothing() {
    for c in [channel::STOREFRONT, channel::CONSOLE, channel::WHATSAPP, channel::INSTAGRAM, channel::EBILLS, "WOLT"] {
        let mut e = entry(1600);
        e.channel = c.into();
        assert!(matches!(built(&e), Err(Refused::Invalid(_))), "{c}");
    }
    // Twin: each marketplace builds, and stores the set's own word.
    for c in [channel::WOLT, channel::GLOVO, channel::BABOON] {
        let mut e = entry(1600);
        e.channel = c.into();
        let (id, o, _) = built(&e).expect(c);
        assert_eq!((id, o["channel"].clone()), (format!("{c}-W-8841"), json!(c)));
    }
}

/// THE SAME NUMBER TWICE: same content is the same entry (idempotent, even if
/// the lines were typed in another order); a different basket, discount or
/// total is a named 409, never a silent return of the first.
#[test]
fn the_same_number_with_other_content_is_a_conflict_and_the_same_content_is_not() {
    let (_, first, _) = built(&entry(1600)).unwrap();
    let stored = first.to_string();
    // Twin: the very same entry, and the same lines in another order.
    assert!(same_entry(&stored, &built(&entry(1600)).unwrap().1.to_string()).is_ok());
    let mut swapped = entry(1600);
    swapped.lines.reverse();
    assert!(same_entry(&stored, &built(&swapped).unwrap().1.to_string()).is_ok(), "line order is not content");
    // The stored order moved on (status, eta): still the same entry.
    let mut advanced = first.clone();
    advanced["status"] = json!("PREPARING");
    assert!(same_entry(&advanced.to_string(), &stored).is_ok());
    // Refusals: another total (with its discount), another quantity, another price.
    let mut disc = entry(1650);
    disc.discount = 100;
    let mut qty = entry(2300);
    qty.lines[0].quantity = 3;
    let mut price = entry(1700);
    price.lines[1].unit_price = 450;
    for (why, e) in [("discount", disc), ("quantity", qty), ("price", price)] {
        let r = same_entry(&stored, &built(&e).unwrap().1.to_string());
        assert!(matches!(&r, Err(Refused::Conflict(m)) if m.contains("already entered")), "{why}: {r:?}");
        assert_eq!(r.unwrap_err().status(), 409, "{why}");
    }
    assert!(matches!(same_entry("not json", &stored), Err(Refused::Conflict(_))), "unreadable is not 'same'");
}
