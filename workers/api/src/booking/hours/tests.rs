use super::*;
use serde_json::json;

const TIRANE: &str = "Europe/Tirane";

fn zone() -> Zone {
    tz::from_settings(Some(TIRANE), None)
}

/// Minutes since the epoch of a UTC wall time.
fn utc_min(y: i32, mo: u32, d: u32, h: i64, mi: i64) -> i64 {
    let days = days_from_civil(y as i64, mo as i64, d as i64);
    days * 1440 + h * 60 + mi
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

/// Mon-Sat 12:00-23:00, Sunday closed.
fn week() -> Schedule {
    let open = json!([{ "open": 720, "close": 1380 }]);
    schedule_of(Some(&json!([open, open, open, open, open, open, []])))
}

/// THE DEFECT: 03:00 on a closed day was booked. Now refused, both halves.
#[test]
fn three_in_the_morning_and_a_closed_day_are_refused() {
    // Mon 26 Oct 2026 03:00 CET = 02:00Z.
    let why = slot_verdict(&week(), zone(), utc_min(2026, 10, 26, 2, 0)).expect("03:00 must refuse");
    assert!(why.contains("03:00") && why.contains("12:00-23:00"), "{why}");
    // Sunday 25 Oct 19:00 CET = 18:00Z: the venue is shut all day.
    let why = slot_verdict(&week(), zone(), utc_min(2026, 10, 25, 18, 0)).expect("Sunday must refuse");
    assert!(why.contains("no bookings on that day"), "{why}");
}

/// The positive twin, ACROSS THE CHANGE: 19:00 venue time is 17:00Z in
/// summer and 18:00Z in winter, and both are accepted -- an offset read
/// "now" would accept one and refuse the other.
#[test]
fn nineteen_hundred_is_bookable_either_side_of_the_dst_change() {
    assert_eq!(slot_verdict(&week(), zone(), utc_min(2026, 10, 24, 17, 0)), None, "Sat 24 Oct, CEST");
    assert_eq!(slot_verdict(&week(), zone(), utc_min(2026, 10, 26, 18, 0)), None, "Mon 26 Oct, CET");
    // and 17:00Z on the 26th is 18:00 local -- on the grid too, but a
    // different time; 17:15Z (18:15) is off the grid.
    assert!(slot_verdict(&week(), zone(), utc_min(2026, 10, 26, 17, 15)).unwrap().contains("18:15"));
}

/// THE GRID AND THE LAST SITTING, as `timesOn` offers them.
#[test]
fn the_grid_and_the_last_sitting_match_the_storefront() {
    let mon = |h: i64, m: i64| utc_min(2026, 10, 26, h - 1, m); // CET
    assert_eq!(slot_verdict(&week(), zone(), mon(12, 0)), None, "the opening");
    assert_eq!(slot_verdict(&week(), zone(), mon(22, 0)), None, "the last sitting");
    assert!(slot_verdict(&week(), zone(), mon(22, 30)).is_some(), "30 minutes before close");
    assert!(slot_verdict(&week(), zone(), mon(11, 30)).is_some(), "before the opening");
    assert!(slot_verdict(&week(), zone(), mon(19, 10)).is_some(), "off the half hour");
}

/// A kitchen past midnight, a window from 11:15, and no hours on file.
#[test]
fn late_windows_odd_openings_and_the_fallback() {
    let late = json!([{ "open": 1080, "close": 120 }]);
    let s = schedule_of(Some(&json!([late, late, late, late, late, late, late])));
    let mon = |h: i64, m: i64| utc_min(2026, 10, 26, h - 1, m);
    assert_eq!(slot_verdict(&s, zone(), mon(23, 30)), None, "18:00-02:00 takes 23:30");
    assert!(slot_verdict(&s, zone(), mon(17, 30)).is_some());
    let odd = json!([{ "open": 675, "close": 1380 }]);
    let s = schedule_of(Some(&json!([odd, odd, odd, odd, odd, odd, odd])));
    assert_eq!(slot_verdict(&s, zone(), mon(11, 45)), None, "the grid runs from 11:15");
    assert!(slot_verdict(&s, zone(), mon(12, 0)).is_some());
    // No hours filed: the storefront's 11:00-23:00, every day.
    for none in [None, Some(&Value::Null), Some(&json!([]))] {
        let s = schedule_of(none);
        assert_eq!(slot_verdict(&s, zone(), mon(11, 0)), None);
        assert!(slot_verdict(&s, zone(), mon(22, 30)).is_some());
    }
}
