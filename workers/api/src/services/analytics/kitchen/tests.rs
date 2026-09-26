//! The window in the venue's days, and the catalogue as the report reads it.

use super::*;

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's zone")
}
/// 2026-09-26 10:00 UTC = 12:00 in Tirane.
const NOW: i64 = 1_790_416_800_000;

#[test]
fn the_default_window_is_the_last_seven_venue_days() {
    let w = window(tirane(), NOW, None, None, None).unwrap();
    assert_eq!(w.days.len(), 7);
    assert_eq!((w.days[0], w.days[6]), (20260920, 20260926));
    for s in &w.starts {
        assert_eq!(dowiz_hub::tz::start_of_local_day_ms(tirane(), *s), *s, "a local midnight");
    }
    assert!(w.bucket(NOW) == Some(6) && w.bucket(w.end) .is_none() && w.bucket(w.starts[0] - 1).is_none());
}

/// A range: named days, the day the clocks go back (25 hours) is one day,
/// and what is not a range is a 400's text. Twin: the longest allowed passes.
#[test]
fn a_named_range_and_what_is_refused() {
    let w = window(tirane(), NOW, Some("2026-10-24"), Some("2026-10-26"), None).unwrap();
    assert_eq!(w.days, vec![20261024, 20261025, 20261026]);
    assert_eq!(w.starts[2] - w.starts[1], 25 * 3_600_000, "25 October is 25 hours long");
    assert!(window(tirane(), NOW, Some("2026-09-27"), Some("2026-09-26"), None).is_err());
    assert!(window(tirane(), NOW, Some("2026-13-01"), None, None).is_err());
    assert!(window(tirane(), NOW, Some("2026-07-01"), Some("2026-09-26"), None).unwrap_err().contains("62"));
    assert_eq!(window(tirane(), NOW, Some("2026-07-27"), Some("2026-09-26"), None).unwrap().days.len(), 62);
    // `days` counts back from `to` (today by default); outside 1..62 it is refused.
    assert_eq!(window(tirane(), NOW, None, None, Some("30")).unwrap().days[0], 20260828);
    assert_eq!(window(tirane(), NOW, None, Some("2026-09-10"), Some("1")).unwrap().days, vec![20260910]);
    for bad in ["0", "63", "x"] {
        assert!(window(tirane(), NOW, None, None, Some(bad)).is_err(), "{bad}");
    }
}

/// The catalogue read: supplies with their basis, list price and losses;
/// dishes with their recipe weighed (gross, net, out).
#[test]
fn the_catalogue_as_the_report_reads_it() {
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_supply("salmon", r#"{"id":"salmon","name":"Salmon","unit":"g","kind":"food_ingredient","costPerBasis":180,"cleanPm":550}"#);
    cat.set_supply("box", r#"{"id":"box","name":"Box","unit":"unit","kind":"packaging","weightPerUnit":12}"#);
    cat.set_product("sake", r#"{"id":"sake","name":"Sake roll","bom":[{"supply":"salmon","qty":100,"out":50},{"supply":"box","qty":1}]}"#);
    cat.set_product("water", r#"{"id":"water","name":"Water"}"#);
    let s = supplies_of(&cat);
    assert_eq!((s["salmon"].basis, s["salmon"].list_cost, s["salmon"].clean_pm, s["salmon"].cook_pm), (100, Some(180), 550, 1000));
    assert_eq!(s["box"].basis, 1);
    let d = dishes_of(&cat);
    assert_eq!(d["sake"].lines[0], DishLine { supply: "salmon".into(), qty: 100, gross_g: Some(100), net_g: Some(55), out_g: Some(50) });
    assert_eq!(d["sake"].lines[1].gross_g, Some(12));
    assert!(d["water"].lines.is_empty());
}
