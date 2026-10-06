//! W-STORE2 through the stock door's turn: a station bound and unbound, the
//! refusals that name the storage (400), the storage list's `stations`, and a
//! freezing record's start typed in the venue's local time.
use super::*;
use crate::services::operations::stock::storages::list;
use crate::services::operations::stock::turn::{run as turn, supplies_of, StockTurnIn};
use dowiz_hub::tz::{from_settings, Zone};

const NOW: i64 = 1_790_000_000_000;

fn input(kind: &str, body: Value) -> StockTurnIn {
    let supplies = supplies_of(vec![("salmon".into(), json!({ "name": "Salmon", "unit": "g" }).to_string())]);
    StockTurnIn { kind: kind.into(), body, by: "p_anna".into(), now_ms: NOW, today: 20260926, supplies, currency: String::new() }
}

#[test]
fn a_station_is_bound_to_an_open_storage_and_listed_on_it() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, &input("storage", json!({ "card": { "name": "Cellar" } })), false).unwrap();
    let (shown, told) = turn(&mut log, &input(BOUND, json!({ "station": "bar", "store": "cellar" })), false).unwrap();
    assert_eq!((shown["station"].as_str(), shown["store"].as_str(), told.len()), (Some("bar"), Some("cellar"), 0));
    let all = list(&log);
    let cellar = all.as_array().unwrap().iter().find(|s| s["id"] == "cellar").unwrap();
    assert_eq!(cellar["stations"], json!(["bar"]));
    assert!(all[0].get("stations").is_none(), "the kitchen has none bound: no key, the old bytes");
    // Refused with a 400 that NAMES the storage, nothing written.
    let n = log.len();
    let (status, said) = turn(&mut log, &input(BOUND, json!({ "station": "sushi", "store": "attic" })), false).unwrap_err();
    assert_eq!(status, 400);
    assert!(said.contains("attic"), "{said}");
    turn(&mut log, &input("storage", json!({ "card": { "id": "s9", "name": "Old", "archived": true } })), false).unwrap();
    let n2 = log.len();
    let (status, said) = turn(&mut log, &input(BOUND, json!({ "station": "sushi", "store": "s9" })), false).unwrap_err();
    assert_eq!((status, said.contains("s9")), (400, true), "{said}");
    assert_eq!(turn(&mut log, &input(BOUND, json!({ "station": "grill", "store": "cellar" })), false).unwrap_err().0, 400);
    assert_eq!(log.len(), n2);
    assert!(n2 > n);
    // Archiving the bound cellar is refused; unbound, the binding is gone.
    let archive = input("storage", json!({ "card": { "id": "cellar", "name": "Cellar", "archived": true } }));
    assert_eq!(turn(&mut log, &archive, false).unwrap_err().0, 409);
    turn(&mut log, &input(BOUND, json!({ "station": "bar", "store": "" })), false).unwrap();
    turn(&mut log, &archive, false).unwrap();
    assert!(log.bindings().unwrap().is_empty());
}

/// Tirana in October is UTC+2 (summer time); in December UTC+1.
fn tirana() -> Zone {
    from_settings(Some("Europe/Tirane"), None)
}

#[test]
fn a_start_in_local_time_becomes_the_instant_and_decides_the_rule() {
    assert_eq!(parse_local("2026-10-05T14:30").map(|l| l % 86_400_000), Some(14 * 3_600_000 + 30 * 60_000));
    assert_eq!(parse_local("2026-10-05T24:00"), None);
    assert_eq!(parse_local("2026-10-05T14:30:10"), None, "minutes precision");
    assert_eq!(parse_local("yesterday"), None);
    let summer = local_to_utc(tirana(), parse_local("2026-10-05T14:30").unwrap());
    let winter = local_to_utc(tirana(), parse_local("2026-12-05T14:30").unwrap());
    assert_eq!(dowiz_hub::tz::local_ms(tirana(), summer) % 86_400_000, 14 * 3_600_000 + 30 * 60_000);
    assert_eq!(dowiz_hub::tz::local_ms(tirana(), winter) % 86_400_000, 14 * 3_600_000 + 30 * 60_000);
    assert_eq!(dowiz_hub::tz::offset_minutes(tirana(), summer) - dowiz_hub::tz::offset_minutes(tirana(), winter), 60);
    // The object stamps it; whatever a body carried under the key is dropped.
    let mut body = json!({ "item": "salmon", "lot": "L1", "tempC": -20, "started": "2026-10-05T14:30", "startedMs": 1 });
    stamp_started(&mut body, tirana());
    assert_eq!(body[STARTED_MS], json!(summer));
    // 20 h before NOW: typed 24 h, decided 20 h -> no rule; hours may be left out.
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let b = json!({ "item": "salmon", "lot": "L1", "hours": 24, "tempC": -20, "started": "x", "startedMs": NOW - 20 * 3_600_000 });
    let (shown, _) = turn(&mut log, &input("frozen", b), false).unwrap();
    assert_eq!((shown["hours"].clone(), shown["typedHours"].clone(), shown["rule"].clone()), (json!(20), json!(24), Value::Null));
    let b = json!({ "item": "salmon", "lot": "L1", "tempC": -20, "started": "x", "startedMs": NOW - 25 * 3_600_000 });
    let (shown, _) = turn(&mut log, &input("frozen", b), false).unwrap();
    assert_eq!((shown["hours"].clone(), shown["rule"].clone()), (json!(25), json!("-20C/24h")));
    // Text the object could not read is a 400, nothing written.
    let n = log.len();
    let (status, said) = turn(&mut log, &input("frozen", json!({ "item": "salmon", "lot": "L1", "tempC": -20, "started": "soon" })), false).unwrap_err();
    assert_eq!(status, 400);
    assert!(said.contains("soon"), "{said}");
    assert_eq!(log.len(), n);
}

/// OPERATOR 2026-10-05: a recorded END (venue local, stamped like the start)
/// decides with the start: 20 h of freezing written down 10 h later meets
/// nothing, and an end the object could not read is a 400.
#[test]
fn a_recorded_end_keeps_a_late_record_honest() {
    let mut body = json!({ "started": "2026-10-04T08:00", "ended": "2026-10-05T04:00", "endedMs": 1 });
    stamp_started(&mut body, tirana());
    assert_eq!(body[ENDED_MS].as_i64().unwrap() - body[STARTED_MS].as_i64().unwrap(), 20 * 3_600_000);
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let h = 3_600_000;
    let b = json!({ "item": "salmon", "lot": "L1", "tempC": -20, "started": "x", "startedMs": NOW - 30 * h, "ended": "y", "endedMs": NOW - 10 * h });
    let (shown, _) = turn(&mut log, &input("frozen", b), false).unwrap();
    assert_eq!((shown["hours"].clone(), shown["rule"].clone(), shown["ended"].clone()), (json!(20), Value::Null, json!(NOW - 10 * h)));
    let n = log.len();
    let bad = json!({ "item": "salmon", "lot": "L1", "tempC": -20, "started": "x", "startedMs": NOW - 30 * h, "ended": "later" });
    let (status, said) = turn(&mut log, &input("frozen", bad), false).unwrap_err();
    assert_eq!((status, said.contains("later")), (400, true), "{said}");
    assert_eq!(log.len(), n);
}
