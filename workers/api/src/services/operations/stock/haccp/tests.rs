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
fn nobody(_: &str) -> String {
    String::new()
}
fn minute(ms: i64) -> String {
    format!("m{ms}")
}
fn names() -> Names<'static> {
    Names { supply: &name, staff: &nobody, local_minute: &minute }
}

#[test]
fn the_csv_finds_an_order_under_its_lot() {
    let t = trace();
    let lots = csv("lots", &t, (20261005, 20261005), &day, &names()).unwrap();
    assert_eq!(
        lots,
        "item,name,lot,date,order,qty\nsalmon,\"Salmon, raw\",#1,2026-10-05,o-7,50\nsalmon,\"Salmon, raw\",L-soon,2026-10-05,o-7,150\n"
    );
    let orders = csv("orders", &t, (20261001, 20261031), &day, &names()).unwrap();
    assert!(orders.contains("o-7,2026-10-05,salmon,\"Salmon, raw\",L-soon,150\n"), "{orders}");
    // Outside the range: no draw.
    assert_eq!(csv("lots", &t, (20261006, 20261010), &day, &names()).unwrap(), "item,name,lot,date,order,qty\n");
    let freezing = csv("freezing", &t, (20261004, 20261004), &day, &names()).unwrap();
    assert_eq!(freezing.lines().nth(1), Some("2026-10-04,salmon,\"Salmon, raw\",L-soon,supplier,,,,CERT-77,,,,,"));
    assert!(csv("everything", &t, (20261004, 20261004), &day, &names()).is_err());
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

/// W-STORE2: the freezing log shows the staff id AND the name, resolved at
/// export time; a member since removed prints the id with an empty name and
/// the export still succeeds. The start is a column; an old record leaves it
/// empty. Over the object's own path (`fold`), the names come from `names=`.
#[test]
fn the_freezing_log_names_its_signer_and_its_start() {
    use dowiz_hub::stock::haccp::Frozen;
    const H: i64 = 3_600_000;
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let f = |by: &str, started: Option<i64>| Frozen {
        item: "salmon".into(), lot: "L1".into(), at: 1_000 + 30 * H, hours: 24, temp_c: -20, store: "freezer".into(), by: by.into(), started, ended: None,
    };
    log.record_frozen(&f("p_anna", Some(1_000 + 10 * H))).unwrap();
    log.record_frozen(&f("p_gone", None)).unwrap();
    let t = log.trace().unwrap();
    let staff = |id: &str| if id == "p_anna" { "Anna, cook".to_string() } else { String::new() };
    let n = Names { supply: &name, staff: &staff, local_minute: &minute };
    let out = csv("freezing", &t, (20261001, 20261031), &|_| 20261005, &n).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "date,item,name,lot,how,hours,temp_c,rule,doc,storage,by,by_name,started,ended");
    let started = 1_000 + 10 * H;
    assert_eq!(lines[1], format!("2026-10-05,salmon,\"Salmon, raw\",L1,in_house,20,-20,none,,freezer,p_anna,\"Anna, cook\",m{started},"), "20 h from start to end: no rule");
    assert_eq!(lines[2], "2026-10-05,salmon,\"Salmon, raw\",L1,in_house,24,-20,-20C/24h,,freezer,p_gone,,,", "a removed member: id, empty name");
    // The object's door: names arrive as JSON; garbage is no names, never an error.
    assert_eq!(staff_map(Some(r#"{"p_anna":"Anna"}"#)).get("p_anna").map(String::as_str), Some("Anna"));
    assert!(staff_map(Some("not json")).is_empty());
    assert!(staff_map(None).is_empty());
}
