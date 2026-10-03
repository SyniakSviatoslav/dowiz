//! `stock.digest` through the venue object's turn: a count that closes a
//! tracked window tells the biggest unexplained losses once a week; an
//! untracked supply, a first count, and a second count in the week tell nothing.

use super::super::turn::{run, supplies_of, StockTurnIn, Told};
use super::*;
use std::collections::BTreeMap;

const NOW: i64 = 1_790_000_000_000;

fn supplies(linked: bool) -> BTreeMap<String, super::super::turn::SupplyIn> {
    let mut s = supplies_of(vec![
        ("salmon".into(), json!({ "name": "Salmon", "unit": "g" }).to_string()),
        ("rice".into(), json!({ "name": "Rice", "unit": "g" }).to_string()),
    ]);
    s.get_mut("salmon").unwrap().linked = linked;
    s
}

fn turn(log: &mut StockLog, kind: &str, body: Value, now: i64, linked: bool) -> Told {
    let input = StockTurnIn { kind: kind.into(), body, by: "p_anna".into(), now_ms: now, today: 20260926, supplies: supplies(linked), currency: "ALL".into() };
    run(log, &input, false).unwrap().1
}

fn digest_of(t: &Told) -> Option<&Value> {
    t.iter().find(|(k, _)| *k == EVENT).map(|(_, v)| v)
}

fn count(log: &mut StockLog, observed: i64, now: i64, linked: bool) -> Told {
    turn(log, "count", json!({ "lines": [{ "item": "salmon", "observed": observed }, { "item": "rice", "observed": 500 }] }), now, linked)
}

fn shelf() -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, "received", json!({ "item": "salmon", "qty": 1000, "unitCost": 2000, "per": 1000 }), NOW - 10, true);
    log
}

#[test]
fn a_count_that_closes_a_tracked_window_tells_its_loss_once_a_week() {
    let mut log = shelf();
    assert!(digest_of(&count(&mut log, 1000, NOW, true)).is_none(), "the first count opens a window, closes none");
    let told = count(&mut log, 900, NOW + 1, true);
    let d = digest_of(&told).expect("due: no digest this week yet");
    assert_eq!(d, &json!({ "items": [{ "name": "Salmon", "qty": 100, "unit": "g", "value": 200 }], "currency": "ALL", "more": 0, "checked": 1 }));
    assert_eq!(log.notes(NOTE).len(), 1, "the log remembers it");
    assert!(digest_of(&count(&mut log, 850, NOW + 2, true)).is_none(), "the same week: not again");
    let later = count(&mut log, 800, NOW + 2 + EVERY_MS, true);
    assert_eq!(digest_of(&later).unwrap()["items"][0]["qty"], json!(50), "a week on: the window since the last count");
}

#[test]
fn an_untracked_supply_is_never_a_loss_and_tells_nothing() {
    let mut log = shelf();
    count(&mut log, 1000, NOW, false);
    let told = count(&mut log, 900, NOW + 1, false);
    assert!(digest_of(&told).is_none(), "no recipe names salmon: not tracked");
    assert!(log.notes(NOTE).is_empty(), "and nothing is remembered");
    assert!(told.iter().any(|(k, _)| *k == "stocktake.variance"), "the count's own drift is still told");
}

#[test]
fn a_tracked_count_with_nothing_lost_says_so() {
    let mut log = shelf();
    count(&mut log, 1000, NOW, true);
    let d = digest_of(&count(&mut log, 1000, NOW + 1, true)).cloned().expect("a window was closed");
    assert_eq!(d["items"], json!([]));
    assert!(text(&d, "en").contains("Nothing unexplained"));
}

#[test]
fn due_reads_the_newest_note() {
    let n = |at: i64| format!(r#"{{"k":"note","note":"digest","at":{at}}}"#);
    assert!(due(&[], NOW));
    assert!(!due(&[n(NOW - 86_400_000)], NOW));
    assert!(due(&[n(NOW - EVERY_MS)], NOW));
    assert!(!due(&[n(NOW - EVERY_MS), n(NOW - 5)], NOW), "the newest decides");
}

#[test]
fn the_text_in_each_language_with_money() {
    let d = json!({ "items": [{ "name": "Salmon", "qty": 100, "unit": "g", "value": 200 }, { "name": "Nori", "qty": 3, "unit": "unit" }], "currency": "ALL", "more": 2 });
    let en = text(&d, "en");
    assert!(en.starts_with("🧾 Weekly stock check"), "{en}");
    assert!(en.contains(&format!("1. Salmon: 100 g · {}", crate::notify::money_text(200, "ALL"))), "{en}");
    assert!(en.contains("2. Nori: 3 unit\n") && en.contains("+2\n") && en.ends_with("Ingredients > Losses."), "{en}");
    assert!(text(&d, "sq").contains("Kontrolli i javës") && text(&d, "uk").contains("Тижнева") && text(&d, "ru").contains("Еженедельная"));
    assert_eq!(text(&d, "de"), en, "a language nobody wrote reads English");
}
