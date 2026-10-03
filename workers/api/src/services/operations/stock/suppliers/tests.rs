//! Supplier cards and orders sent, through the venue object's turn: a card is
//! checked and stored as a note, edited by writing it again, taken off by
//! `gone`; an order is open until deliveries after it close it, oldest first.

use super::super::turn::{run, supplies_of, StockTurnIn};
use super::*;

const NOW: i64 = 1_790_000_000_000;
const DAY: i64 = 86_400_000;

fn input(kind: &str, card: Value, now: i64) -> StockTurnIn {
    let supplies = supplies_of(vec![
        ("salmon".into(), json!({ "name": "Salmon", "unit": "g" }).to_string()),
        ("rice".into(), json!({ "name": "Rice", "unit": "g" }).to_string()),
    ]);
    StockTurnIn { kind: kind.into(), body: json!({ "card": card }), by: "p_anna".into(), now_ms: now, today: 20260926, supplies, currency: String::new() }
}

fn log() -> StockLog {
    StockLog::create_sized(64 * 1024).unwrap()
}

fn card(log: &mut StockLog, c: Value) -> Result<Value, Bad> {
    run(log, &input(CARD, c, NOW), false).map(|r| r.0)
}

#[test]
fn a_card_is_checked_minted_and_edited_by_writing_it_again() {
    let mut l = log();
    let shown = card(&mut l, json!({ "name": " Peshku i Detit ", "phone": "+355 69 123 4567", "days": [4, 1, 4], "leadDays": 1, "cutoff": "18:00", "lang": "sq" })).unwrap();
    assert_eq!(shown["card"]["id"], "peshku-i-detit", "the id is minted from the name");
    let c = &cards(&l)[0];
    assert_eq!((c.name.as_str(), c.days.clone(), c.lead_days, c.lang.as_str()), ("Peshku i Detit", vec![1, 4], 1, "sq"), "days sorted, once each");
    card(&mut l, json!({ "id": "peshku-i-detit", "name": "Peshku i Detit", "leadDays": 2 })).unwrap();
    assert_eq!(cards(&l).len(), 1, "same id: the newest card is the card");
    assert_eq!(cards(&l)[0].lead_days, 2);
    assert_eq!(l.ledger().unwrap(), dowiz_hub::stock::StockLedger::default(), "a card moves no stock");
    card(&mut l, json!({ "id": "peshku-i-detit", "name": "Peshku i Detit", "gone": true })).unwrap();
    assert!(cards(&l).is_empty(), "gone: off the list");
}

#[test]
fn a_card_that_is_not_one_is_refused_and_nothing_is_written() {
    let mut l = log();
    for bad in [
        json!({ "name": "" }),
        json!({ "name": "x".repeat(NAME_MAX + 1) }),
        json!({ "name": "A", "phone": "call me" }),
        json!({ "name": "A", "days": [0] }),
        json!({ "name": "A", "days": [8] }),
        json!({ "name": "A", "leadDays": LEAD_MAX + 1 }),
        json!({ "name": "A", "leadDays": -1 }),
        json!({ "name": "A", "cutoff": "25:00" }),
        json!({ "name": "A", "lang": "de" }),
        json!({ "name": "A", "rating": 5 }),
    ] {
        assert!(card(&mut l, bad.clone()).is_err(), "{bad} refused");
    }
    assert!(run(&mut l, &StockTurnIn { body: json!({ "name": "A" }), ..input(CARD, Value::Null, NOW) }, false).is_err(), "the card is under `card`");
    assert_eq!(l.len(), 0, "nothing written");
    // The positive twin: the plainest card there is.
    assert!(card(&mut l, json!({ "name": "A" })).is_ok());
    assert_eq!(l.len(), 1);
}

fn ordered(l: &mut StockLog, lines: Value, now: i64) -> Result<Value, Bad> {
    run(l, &input(ORDERED, json!({ "supplier": "sea", "lines": lines }), now), false).map(|r| r.0)
}

fn receive(l: &mut StockLog, item: &str, qty: i64, now: i64) {
    run(l, &StockTurnIn { body: json!({ "item": item, "qty": qty }), ..input("received", Value::Null, now) }, false).unwrap();
}

#[test]
fn an_order_is_open_until_deliveries_after_it_close_it_oldest_first() {
    let mut l = log();
    card(&mut l, json!({ "name": "Sea", "id": "sea" })).unwrap();
    receive(&mut l, "salmon", 300, NOW - DAY); // before any order: closes nothing
    ordered(&mut l, json!([{ "item": "salmon", "qty": 2000 }, { "item": "rice", "qty": 5000 }]), NOW).unwrap();
    ordered(&mut l, json!([{ "item": "salmon", "qty": 1000 }]), NOW + 10).unwrap();
    let open = |l: &StockLog, now: i64| on_order(&l.notes(ORDERED), &l.journal().unwrap(), now);
    assert_eq!(open(&l, NOW + 20)["salmon"], Open { qty: 3000, since: Some(NOW) });
    receive(&mut l, "salmon", 2500, NOW + 30);
    assert_eq!(open(&l, NOW + 40)["salmon"], Open { qty: 500, since: Some(NOW + 10) }, "the first order closed, 500 of the second left");
    receive(&mut l, "salmon", 900, NOW + 50);
    assert!(!open(&l, NOW + 60).contains_key("salmon"), "more than ordered: closed, never negative");
    assert_eq!(open(&l, NOW + 60)["rice"].qty, 5000);
    assert!(!open(&l, NOW + ORDER_OPEN_DAYS * DAY + 1).contains_key("rice"), "never delivered: forgotten after two weeks");
}

#[test]
fn an_order_names_a_card_and_known_supplies() {
    let mut l = log();
    assert_eq!(ordered(&mut l, json!([{ "item": "salmon", "qty": 1 }]), NOW).unwrap_err().0, 404, "no card yet");
    card(&mut l, json!({ "name": "Sea", "id": "sea" })).unwrap();
    let n = l.len();
    assert_eq!(ordered(&mut l, json!([{ "item": "tuna", "qty": 1 }]), NOW).unwrap_err(), (404, "not found: tuna".to_string()));
    assert_eq!(ordered(&mut l, json!([{ "item": "salmon", "qty": 0 }]), NOW).unwrap_err().0, 400);
    assert_eq!(ordered(&mut l, json!([]), NOW).unwrap_err().0, 400);
    assert_eq!(ordered(&mut l, json!([{ "item": "salmon", "qty": 1, "price": 9 }]), NOW).unwrap_err().0, 400, "a line is item and qty");
    assert_eq!(l.len(), n, "a refused order writes nothing");
    let ok = ordered(&mut l, json!([{ "item": "salmon", "qty": 1 }]), NOW).unwrap();
    assert_eq!((ok["po"].clone(), l.len()), (json!(format!("po_{NOW}")), n + 1));
}
