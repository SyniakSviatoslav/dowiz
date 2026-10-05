//! A13's laws (W-LOST): a refusal moves no shelf number, an old image folds
//! as before, the rate limit is one per supply per ten minutes, no personal
//! field can reach the record, and a refused basket yields exactly one row.

use super::super::meta::Meta;
use super::super::oldimage_tests::{fold_text, old_history};
use super::super::*;
use super::{lost_of, on_refusal, row_of, LostSale, REFUSED, WINDOW_MS};

const T0: i64 = 1_759_000_000_000;
const MIN: i64 = 60_000;

fn sale(item: &str) -> LostSale {
    LostSale { item: item.into(), dish: "philadelphia".into(), qty: 2, price: 1200 }
}

fn log_at(now: i64) -> StockLog {
    let mut l = StockLog::create_sized(16 * 1024).unwrap();
    l.set_clock(now);
    l
}

/// The P12 golden's history, with a refusal (two supplies) after every step
/// when `refuse`: the same writes otherwise, in the same order, at the same clock.
fn history(refuse: bool) -> StockLog {
    let mut log = StockLog::create_sized(16 * 1024).unwrap();
    log.set_checkpoint_every(7);
    for step in 0..40i64 {
        log.set_clock(T0 + step * 3_600_000);
        let m = Meta { unit_cost: Some(900 + step * 10), per: Some(1000), lot: (step % 3 == 0).then(|| format!("L{step}")), expiry: Some(20261010 + step % 9), ..Meta::default() };
        log.receive_with("salmon", 1500 + step, &m).unwrap();
        log.receive_with("rice", 5000, &Meta::default()).unwrap();
        let order = format!("o{step}");
        log.append_all(&reservations_for(&order, &[(r#"{"bom":[{"supply":"salmon","qty":40},{"supply":"rice","qty":90}]}"#.into(), 3)])).unwrap();
        if refuse {
            assert!(log.append_lost(&sale("salmon")).unwrap(), "an hour apart: written");
            assert!(log.append_lost(&sale("rice")).unwrap(), "another supply: written");
        }
        let led = log.ledger().unwrap();
        log.append_all(&settle(&led, &order, step % 4 != 0)).unwrap();
        if step % 5 == 0 {
            log.append(&StockEvent::Wasted { item: "salmon".into(), qty: 30, reason: WasteReason::Spoiled, by: "p1".into() }).unwrap();
        }
    }
    log
}

#[test]
fn a_refused_row_moves_no_shelf_number() {
    let (with, without) = (history(true), history(false));
    assert_eq!(with.lost_rows().len(), 80, "every refusal is in the chain");
    assert_eq!(with.ledger().unwrap(), without.ledger().unwrap(), "the shelf, through the checkpoints");
    assert_eq!(with.events(), without.events(), "a refusal is not an event");
    let (a, b) = (with.journal().unwrap(), without.journal().unwrap());
    assert_eq!(fold_text(&a), fold_text(&b), "levels, cost book, lots, carry");
    assert_eq!(format!("{:?}", a.stores), format!("{:?}", b.stores), "the storages");
    assert_eq!(a.entries, b.entries, "every journal row, seq included");
    assert_eq!(with.cost_book(), without.cost_book());
    assert!(with.verify_checkpoints().unwrap() > 5, "every checkpoint written beside refusals agrees with the fold");
}

#[test]
fn an_old_image_reads_no_refusal_and_folds_the_same() {
    let old = old_history();
    let reloaded = StockLog::load(&old.to_bytes_trimmed()).unwrap();
    assert!(reloaded.lost_rows().is_empty(), "a log written before refusals has none");
    assert_eq!(fold_text(&reloaded.journal().unwrap()), fold_text(&old.journal().unwrap()));
    // And the other direction: a log WITH refusals, read by a decoder that
    // does not know them (`decode`), is the same history of events.
    let with = history(true);
    let raw_events: Vec<StockEvent> = with.raw().iter().filter_map(|r| decode(r)).collect();
    assert_eq!(raw_events, history(false).events());
}

#[test]
fn the_rate_limit_is_one_per_supply_per_ten_minutes() {
    let mut l = log_at(T0);
    assert!(l.append_lost(&sale("salmon")).unwrap(), "the first is written");
    l.set_clock(T0 + 9 * MIN);
    assert!(!l.append_lost(&sale("salmon")).unwrap(), "9 minutes later: the same lost sale");
    assert!(l.append_lost(&sale("rice")).unwrap(), "another supply is its own row");
    assert_eq!(l.lost_rows().len(), 2, "2 refusals of salmon in 10 minutes = 1 row");
    l.set_clock(T0 + 11 * MIN);
    assert!(l.append_lost(&sale("salmon")).unwrap(), "11 minutes after the first: a new row");
    let salmon: Vec<i64> = l.lost_rows().iter().filter(|r| r.sale.item == "salmon").map(|r| r.at).collect();
    assert_eq!(salmon, vec![T0, T0 + 11 * MIN], "11 minutes apart = 2 rows, each at its own clock");
    assert_eq!(WINDOW_MS, 10 * MIN);
}

#[test]
fn the_window_is_not_fooled_by_records_between() {
    let mut l = log_at(T0);
    l.append_lost(&sale("salmon")).unwrap();
    for k in 0..50 {
        l.set_clock(T0 + k * 1000);
        l.append(&StockEvent::Received { item: "rice".into(), qty: 10 }).unwrap();
    }
    l.set_clock(T0 + 5 * MIN);
    assert!(!l.append_lost(&sale("salmon")).unwrap(), "fifty records later, still inside the window");
}

#[test]
fn no_personal_field_can_enter_a_refused_record() {
    let mut l = log_at(T0);
    // A basket whose product JSON carries every personal word a request has:
    // the record takes the dish's id and price, nothing else from it.
    let dish = r#"{"id":"maki","price":700,"phone":"+355691234567","guest":"Ana","order":"o-1","bom":[{"supply":"salmon","qty":40}]}"#;
    let e = StockError::OutOfStock { item: "salmon".into(), wanted: 40, available: 30 };
    let back = on_refusal(&mut l, &[(dish.into(), 1)], e);
    assert!(matches!(back, StockError::OutOfStock { .. }), "the refusal is handed back unchanged");
    let recs = l.notes(REFUSED);
    assert_eq!(recs.len(), 1);
    let v: serde_json::Value = serde_json::from_str(&recs[0]).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, vec!["at", "dish", "item", "k", "note", "price", "qty"], "these keys and no others: {}", recs[0]);
    for word in ["355691234567", "Ana", "o-1", "phone", "guest", "order"] {
        assert!(!recs[0].contains(word), "{word} reached the record: {}", recs[0]);
    }
}

#[test]
fn a_refused_basket_yields_exactly_one_row_and_a_placed_one_none() {
    let mut l = log_at(T0);
    l.append(&StockEvent::Received { item: "salmon".into(), qty: 30 }).unwrap();
    let basket = vec![(r#"{"id":"maki","price":700,"bom":[{"supply":"salmon","qty":40}]}"#.to_string(), 1)];
    let before = l.len();
    let err = l.append_draws_split(&draws_for("o1", &basket)).map(|_| ()).map_err(|e| on_refusal(&mut l, &basket, e));
    assert!(matches!(err, Err(StockError::OutOfStock { .. })), "30 g on the shelf, 40 g wanted: refused");
    assert_eq!(l.len(), before + 1, "one record, and it is the refusal");
    let rows = l.lost_rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].sale, LostSale { item: "salmon".into(), dish: "maki".into(), qty: 1, price: 700 });
    // The twin: a basket that fits writes its reservations and no refusal.
    l.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    l.append_draws_split(&draws_for("o2", &basket)).unwrap();
    assert_eq!(l.lost_rows().len(), 1, "a placed basket is no lost sale");
}

#[test]
fn only_out_of_stock_is_a_lost_sale_and_a_refusal_needs_a_clock() {
    let basket = vec![(r#"{"id":"maki","price":700,"bom":[{"supply":"salmon","qty":40}]}"#.to_string(), 1)];
    let mut l = log_at(T0);
    on_refusal(&mut l, &basket, StockError::Linkage("x".into()));
    on_refusal(&mut l, &basket, StockError::Overflow);
    assert!(l.lost_rows().is_empty(), "a refusal that is not a stock-out records nothing");
    let mut clockless = StockLog::create_sized(16 * 1024).unwrap();
    assert!(matches!(clockless.append_lost(&sale("salmon")), Err(StockError::Malformed)), "no clock: refused");
    on_refusal(&mut clockless, &basket, StockError::OutOfStock { item: "salmon".into(), wanted: 40, available: 0 });
    assert_eq!(clockless.len(), 0, "and the placement's refusal stands without it");
    assert!(matches!(l.append_lost(&LostSale { qty: 0, ..sale("salmon") }), Err(StockError::NotPositive { .. })));
    assert!(matches!(l.append_lost(&LostSale { dish: " ".into(), ..sale("salmon") }), Err(StockError::Malformed)));
    assert!(l.append_lost(&sale("salmon")).unwrap(), "the twin: a whole sale is written");
}

#[test]
fn the_dish_is_the_line_that_needed_the_item_and_its_price_counts_the_options() {
    let lines = vec![
        (r#"{"id":"cola","price":200}"#.to_string(), 2),
        (r#"{"id":"maki","price":700,"bom":[{"supply":"rice","qty":90}]}"#.to_string(), 3),
        (r#"{"of":"maki","delta":150,"bom":[{"supply":"salmon","qty":20}]}"#.to_string(), 3),
    ];
    assert_eq!(lost_of(&lines, "salmon"), Some(LostSale { item: "salmon".into(), dish: "maki".into(), qty: 3, price: 850 }), "an option's supply charges its dish");
    assert_eq!(lost_of(&lines, "rice").map(|s| (s.dish, s.price)), Some(("maki".into(), 850)));
    assert_eq!(lost_of(&lines, "nori").map(|s| s.dish), Some("cola".into()), "no line names it: the first dish is charged");
    assert_eq!(lost_of(&[], "nori"), None);
    let nested = vec![(r#"{"modifierGroups":[{"id":"size","options":[{"id":"big"}]}],"id":"roll","price":500,"bom":[{"supply":"nori","qty":1}]}"#.to_string(), 1)];
    assert_eq!(lost_of(&nested, "nori").map(|s| s.dish), Some("roll".into()), "a group's id is never the dish's");
}

#[test]
fn a_record_reads_back_and_a_broken_one_is_skipped() {
    let ok = format!(r#"{{"k":"note","note":"{REFUSED}","item":"salmon","dish":"maki","qty":1,"price":700,"at":5}}"#);
    assert_eq!(row_of(&ok).map(|r| (r.sale.dish, r.at)), Some(("maki".into(), 5)));
    assert_eq!(row_of(r#"{"k":"note","note":"refused","item":"salmon","qty":1,"price":700,"at":5}"#), None, "no dish");
    assert_eq!(row_of(r#"{"k":"note","note":"refused","item":"salmon","dish":"maki","qty":1,"price":700}"#), None, "no clock");
}
