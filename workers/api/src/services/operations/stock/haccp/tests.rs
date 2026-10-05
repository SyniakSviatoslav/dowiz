//! P13: the CSV finds an order under its lot, in the venue's local days, and
//! a spreadsheet cannot be made to run a cell.
use super::*;
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{reservations_for, settle, StockLog};

const ROLL: &str = r#"{"id":"r","bom":[{"supply":"salmon","qty":100}]}"#;

fn trace() -> Trace {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.set_clock(1_000);
    log.receive_with("salmon", 150, &Meta { lot: Some("L-soon".into()), expiry: Some(20261006), treated: Some("CERT-77".into()), ..Meta::default() })
        .unwrap();
    log.receive_with("salmon", 1000, &Meta { expiry: Some(20261020), ..Meta::default() }).unwrap();
    log.set_clock(2_000);
    log.append_all(&reservations_for("o-7", &[(ROLL.into(), 2)])).unwrap();
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, "o-7", true)).unwrap();
    log.trace().unwrap()
}

/// ms 1000 is the venue's 4 October, ms 2000 its 5th.
fn day(ms: i64) -> i64 {
    if ms < 1_500 { 20261004 } else { 20261005 }
}
fn name(_: &str) -> String {
    "Salmon, raw".into()
}

#[test]
fn the_csv_finds_an_order_under_its_lot() {
    let t = trace();
    let lots = csv("lots", &t, (20261005, 20261005), &day, &name).unwrap();
    assert_eq!(
        lots,
        "item,name,lot,date,order,qty\nsalmon,\"Salmon, raw\",#1,2026-10-05,o-7,50\nsalmon,\"Salmon, raw\",L-soon,2026-10-05,o-7,150\n"
    );
    let orders = csv("orders", &t, (20261001, 20261031), &day, &name).unwrap();
    assert!(orders.contains("o-7,2026-10-05,salmon,\"Salmon, raw\",L-soon,150\n"), "{orders}");
    // Outside the range: no draw.
    assert_eq!(csv("lots", &t, (20261006, 20261010), &day, &name).unwrap(), "item,name,lot,date,order,qty\n");
    let freezing = csv("freezing", &t, (20261004, 20261004), &day, &name).unwrap();
    assert_eq!(freezing.lines().nth(1), Some("2026-10-04,salmon,\"Salmon, raw\",L-soon,supplier,,,,CERT-77,,"));
    assert!(csv("everything", &t, (20261004, 20261004), &day, &name).is_err());
}

#[test]
fn a_cell_never_runs_and_a_range_is_days() {
    assert_eq!(cell("=HYPERLINK(\"x\")"), "\"'=HYPERLINK(\"\"x\"\")\"");
    assert_eq!(cell("-20"), "'-20", "text is defused; numbers never pass through cell");
    assert_eq!(cell("plain"), "plain");
    assert_eq!(range(Some("2026-10-01"), Some("2026-10-05")), Ok((20261001, 20261005)));
    assert!(range(Some("2026-10-05"), Some("2026-10-01")).is_err());
    assert!(range(Some("2026-10-05"), None).is_err());
    assert!(range(Some("2025-01-01"), Some("2026-10-01")).is_err(), "more than 400 days");
}
