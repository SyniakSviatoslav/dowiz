//! §2.10's CHECK, through the real log: two receipts at 1,000 and 1,200 per kg
//! give a WAC of 1,100; an order placed between them stamps the 1,000-cost;
//! a rebuild from the persisted bytes reproduces the stamp. Each refusal
//! beside its twin.

use super::*;
use crate::stock::{reservations_for, settle};

const KG: Qty = 1000;

fn price(c: i64) -> Price {
    Price { unit_cost: c, per: KG, supplier: Some("Fish & Co".into()), doc: Some("DN-17".into()) }
}
fn maki() -> Vec<BomLine> {
    vec![BomLine::whole("rice", 100)]
}
const MAKI: &str = r#"{"id":"maki","bom":[{"supply":"rice","qty":100}]}"#;

#[test]
fn two_receipts_average_and_the_order_between_keeps_its_stamp() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", KG, &price(1000)).unwrap();
    assert_eq!(log.cost_book().wac("rice", KG), Some(1000));

    // An order placed between the receipts: 100 g at 1,000 per kg.
    log.append_all(&reservations_for("o1", &[(MAKI.into(), 1)])).unwrap();
    let s = stamp(&log, &maki()).expect("rice has a cost");
    assert_eq!(s.cost, 100);

    log.receive_priced("rice", KG, &price(1200)).unwrap();
    assert_eq!(log.cost_book().wac("rice", KG), Some(1100), "the §2.10 CHECK");
    assert_eq!(stamp(&log, &maki()).unwrap().cost, 110, "a NEW order stamps the new average");

    // LAW 8: from the persisted bytes alone, the old stamp is reproduced.
    let back = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    assert_eq!(rebuild(&back, &s, &maki()), Some(100));
    // Twin: the rebuild is not a constant -- at today's length it says today's.
    let now = Stamp { cost: 0, at: back.len() };
    assert_eq!(rebuild(&back, &now, &maki()), Some(110));
}

#[test]
fn the_shelf_reads_a_priced_receipt_as_the_plain_received_it_always_was() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", 500, &price(1000)).unwrap();
    assert_eq!(log.events(), vec![StockEvent::Received { item: "rice".into(), qty: 500 }]);
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 500);
    assert!(log.raw()[0].contains(r#""supplier":"Fish & Co""#) && log.raw()[0].contains(r#""doc":"DN-17""#));
}

#[test]
fn a_price_that_is_not_one_is_refused_and_nothing_is_written() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for bad in [Price { per: 0, ..price(1000) }, Price { per: -1, ..price(1000) }, price(-5)] {
        assert!(matches!(log.receive_priced("rice", KG, &bad), Err(StockError::NotPositive { .. })), "{bad:?}");
    }
    assert!(log.receive_priced("rice", 0, &price(1000)).is_err(), "the shelf still decides the quantity");
    assert_eq!(log.len(), 0);
    // Twin: a free delivery (cost 0) is a price.
    log.receive_priced("rice", KG, &price(0)).unwrap();
    assert_eq!((log.len(), log.cost_book().wac("rice", KG)), (1, Some(0)));
}

#[test]
fn a_dish_costs_only_when_every_supply_does() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", KG, &price(1000)).unwrap();
    log.append(&StockEvent::Received { item: "nori".into(), qty: 50 }).unwrap();
    let both = vec![BomLine::whole("rice", 100), BomLine::whole("nori", 2)];
    assert_eq!(log.cost_book().dish_cost(&both), None, "nori was never priced");
    assert_eq!(log.cost_book().dish_cost(&[]), None, "no recipe, no cost");
    // Twin: once nori is priced, its unpriced 50 join at that price.
    log.receive_priced("nori", 50, &Price { unit_cost: 30, per: 1, supplier: None, doc: None }).unwrap();
    assert_eq!(log.cost_book().wac("nori", 1), Some(30));
    assert_eq!(log.cost_book().dish_cost(&both), Some(100 + 60));
}

#[test]
fn draws_leave_at_the_average_so_the_next_receipt_weighs_what_is_left() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", KG, &price(1000)).unwrap();
    log.append_all(&reservations_for("o1", &[(MAKI.into(), 5)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o1", true)).unwrap(); // 500 g consumed
    assert_eq!(log.cost_book().wac("rice", KG), Some(1000), "a draw does not move the average");
    log.receive_priced("rice", KG, &price(1200)).unwrap();
    // (500 x 1000 + 1000 x 1200) / 1500 = 1133.3
    assert_eq!(log.cost_book().wac("rice", KG), Some(1133));
    // Twin: a reservation alone draws nothing -- the plain two-receipt 1,100.
    let mut r = StockLog::create_sized(64 * 1024).unwrap();
    r.receive_priced("rice", KG, &price(1000)).unwrap();
    r.append_all(&reservations_for("o1", &[(MAKI.into(), 5)])).unwrap();
    r.receive_priced("rice", KG, &price(1200)).unwrap();
    assert_eq!(r.cost_book().wac("rice", KG), Some(1100));
}

#[test]
fn a_shelf_drawn_to_zero_keeps_its_last_cost() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_priced("rice", 100, &price(1000)).unwrap();
    log.append(&StockEvent::Wasted { item: "rice".into(), qty: 100, reason: crate::stock::WasteReason::Spoiled, by: "p1".into() }).unwrap();
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 0);
    assert_eq!(log.cost_book().wac("rice", KG), Some(1000));
    assert_eq!(StockLog::create_sized(64 * 1024).unwrap().cost_book().wac("rice", KG), None, "twin: never priced");
}

fn paper(cost: i64, at: i64, lot: Option<&str>) -> Meta {
    Meta { unit_cost: Some(cost), per: Some(KG), supplier: Some("Agro".into()), doc: Some("F-9".into()), at: Some(at), lot: lot.map(str::to_string), ..Meta::default() }
}
fn waste(qty: Qty) -> StockEvent {
    StockEvent::Wasted { item: "rice".into(), qty, reason: crate::stock::WasteReason::Spoiled, by: "p1".into() }
}
/// `(code, left, unit_cost, at)` of every open lot of rice, in draw order.
fn lots(log: &StockLog) -> Vec<(String, Qty, Option<i64>, Option<i64>)> {
    log.journal().unwrap().lots.of("rice").iter().map(|l| (l.code.clone(), l.left, l.unit_cost, l.at)).collect()
}

/// ONE POSITION AT TWO PRICES ON ONE PAPER: 10 kg at 500 and 5 kg at 600 per
/// kg. Two records, two lots, one average; what the paper cost is the sum of
/// the two rows, and writing all of it off gives the same money back.
#[test]
fn one_position_at_two_prices_averages_and_every_number_adds_up() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("rice", 10 * KG, &paper(500, 1, None)).unwrap();
    log.receive_with("rice", 5 * KG, &paper(600, 1, None)).unwrap();
    let j = log.journal().unwrap();
    // (10000 x 500 + 5000 x 600) / 15000 = 533.33 per kg.
    assert_eq!(j.book.wac("rice", KG), Some(533));
    assert_eq!(j.book.wac("rice", 100), Some(53), "the list's price per 100 g");
    let bought: Vec<Option<i64>> = j.entries.iter().map(|e| e.value).collect();
    assert_eq!(bought, vec![Some(5000), Some(3000)], "each row is what its line of the paper cost");
    assert_eq!(bought.iter().flatten().sum::<i64>(), 8000, "the purchases total a report reads");
    assert_eq!(lots(&log), vec![("#0".into(), 10 * KG, Some(500), Some(1)), ("#1".into(), 5 * KG, Some(600), Some(1))], "two lots, each with its own price");

    // A later write-off of 3 kg: 3000 x 533.333 = 1599.999, which is 1600.
    log.append(&waste(3 * KG)).unwrap();
    log.append(&waste(12 * KG)).unwrap();
    let j = log.journal().unwrap();
    assert_eq!((j.entries[2].value, j.entries[3].value), (Some(1600), Some(6400)));
    assert_eq!(j.entries[2].value.unwrap() + j.entries[3].value.unwrap(), 8000, "all of it written off is all that was paid");
    // Twin: one price is that price, with nothing to round.
    let mut one = StockLog::create_sized(64 * 1024).unwrap();
    one.receive_with("rice", 15 * KG, &paper(500, 1, None)).unwrap();
    one.append(&waste(3 * KG)).unwrap();
    let j = one.journal().unwrap();
    assert_eq!((j.book.wac("rice", KG), j.entries[1].value), (Some(500), Some(1500)));
}

/// THE SAME LABEL IS ONE BOX: two records under one lot code fold into one lot
/// that shows the FIRST price only (the average and the paper's rows stay
/// right). So a form that takes several prices on a position gives each price
/// its own code; then no lot is merged and no price is hidden.
#[test]
fn two_prices_under_one_lot_code_merge_the_lot_and_distinct_codes_do_not() {
    let mut same = StockLog::create_sized(64 * 1024).unwrap();
    same.receive_with("rice", 10 * KG, &paper(500, 1, Some("L1"))).unwrap();
    same.receive_with("rice", 5 * KG, &paper(600, 1, Some("L1"))).unwrap();
    assert_eq!(lots(&same), vec![("L1".into(), 15 * KG, Some(500), Some(1))], "one lot, the second price not on it");
    let j = same.journal().unwrap();
    assert_eq!(j.book.wac("rice", KG), Some(533), "the average still weighs both");
    assert_eq!(j.entries.iter().filter_map(|e| e.value).sum::<i64>(), 8000);

    let mut apart = StockLog::create_sized(64 * 1024).unwrap();
    apart.receive_with("rice", 10 * KG, &paper(500, 1, Some("L1"))).unwrap();
    apart.receive_with("rice", 5 * KG, &paper(600, 1, Some("L1/2"))).unwrap();
    assert_eq!(lots(&apart), vec![("L1".into(), 10 * KG, Some(500), Some(1)), ("L1/2".into(), 5 * KG, Some(600), Some(1))]);
    assert_eq!(apart.journal().unwrap().book.wac("rice", KG), Some(533));
}

/// PRICES BY DATE: two deliveries of one position on two days at two prices.
/// Each is its own lot with its own day and price; the average weighs them,
/// and a later write-off leaves at that average, off the older lot first.
#[test]
fn two_days_two_prices_keep_a_price_per_date_and_one_average() {
    const DAY1: i64 = 1_762_732_800_000;
    const DAY2: i64 = DAY1 + 86_400_000;
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("rice", 2 * KG, &paper(1800, DAY1, None)).unwrap();
    assert_eq!(log.journal().unwrap().book.wac("rice", KG), Some(1800));
    log.receive_with("rice", KG, &paper(2500, DAY2, None)).unwrap();
    let j = log.journal().unwrap();
    // (2000 x 1800 + 1000 x 2500) / 3000 = 2033.33 per kg.
    assert_eq!(j.book.wac("rice", KG), Some(2033));
    assert_eq!(lots(&log), vec![("#0".into(), 2 * KG, Some(1800), Some(DAY1)), ("#1".into(), KG, Some(2500), Some(DAY2))]);
    // 500 g later: 500 x 2033.333 = 1016.67, which is 1017.
    log.append(&waste(500)).unwrap();
    let j = log.journal().unwrap();
    assert_eq!(j.entries[2].value, Some(1017));
    assert_eq!(lots(&log), vec![("#0".into(), 1500, Some(1800), Some(DAY1)), ("#1".into(), KG, Some(2500), Some(DAY2))]);
    assert_eq!(j.book.wac("rice", KG), Some(2033), "a draw does not move the average");
}
