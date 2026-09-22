//! The rules this server shares with the Worker, each test pinned to the
//! defect it came from. Exempt from the file-size ratchet, which blueprint 2
//! §5 asks for: a gate that counts test files refuses the commit that adds the
//! tests a split was done FOR.

use super::*;
use serde_json::json;

/// 2026-12-30 12:00 UTC, in the middle of the first winter this platform
/// will ever have seen. Written as an instant rather than a parsed date so
/// the test has no date library and no clock either.
const WINTER: i64 = 1_798_632_000_000;
/// 2026-07-15 12:00 UTC — the same venue, the other half of the year.
const SUMMER: i64 = 1_784_116_800_000;
/// 2026-10-26 09:00 UTC — the morning AFTER the long day. 25 October is
/// the 25-hour one: it began at 22:00 UTC on the 24th at +2 and ended at
/// 23:00 UTC on the 25th at +1. A window ending on the 25th is still all
/// 24-hour spans, so this is the day that makes the test bite.
const TRANSITION: i64 = 1_793_005_200_000;

fn default_zone() -> Zone {
    // The env var must not decide a unit test's answer, and this one reads
    // it. Asserting it is unset makes that visible rather than flaky.
    assert!(std::env::var("TZ_OFFSET_MINUTES").is_err(), "the test env sets no offset");
    venue_zone(None)
}

/// THE DEFECT, named: the venue's winter offset is +1, and every one of the
/// four sites in this server said +2 all year.
#[test]
fn the_venues_winter_offset_is_not_its_summer_offset() {
    let z = default_zone();
    assert_eq!(tz::offset_minutes(z, SUMMER), 120, "July is +2");
    assert_eq!(
        tz::offset_minutes(z, WINTER),
        60,
        "December is +1; the shipped constant said 120 all year"
    );
}

/// A venue that names its zone gets ITS zone, not this one's.
#[test]
fn a_venue_that_names_a_zone_gets_it() {
    let kyiv = venue_zone(Some(&json!({ "tz": "Europe/Kyiv" })));
    assert_eq!(tz::offset_minutes(kyiv, WINTER), 120);
    assert_eq!(tz::offset_minutes(kyiv, SUMMER), 180);
    // An explicit fixed offset still works, and never moves.
    let fixed = venue_zone(Some(&json!({ "tz_offset_minutes": 210 })));
    assert_eq!(tz::offset_minutes(fixed, WINTER), 210);
    assert_eq!(tz::offset_minutes(fixed, SUMMER), 210);
    // An unknown name is not silently the default: it falls through to the
    // fixed offset, and the settings route refuses it on write.
    let mars = venue_zone(Some(&json!({ "tz": "Mars/Olympus", "tz_offset_minutes": 60 })));
    assert_eq!(tz::offset_minutes(mars, SUMMER), 60);
}

/// EVERY BOUNDARY IS A REAL LOCAL MIDNIGHT, which is the whole claim. The
/// window is asserted to contain the long day first, or it would prove
/// nothing at all.
#[test]
fn every_day_boundary_in_the_window_is_a_local_midnight() {
    let z = default_zone();
    let starts = day_starts(z, TRANSITION, 7);
    assert_eq!(starts.len(), 7);
    // The window really does straddle the change, or this test is vacuous.
    let spans: Vec<i64> = starts.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        spans.contains(&(25 * 3_600_000)),
        "the window must contain the 25-hour day to prove anything: {spans:?}"
    );
    for b in &starts {
        let local = b + tz::offset_minutes(z, *b) * 60_000;
        assert_eq!(
            local.rem_euclid(DAY_MS),
            0,
            "{b} is not local midnight; it is {} minutes past",
            local.rem_euclid(DAY_MS) / 60_000
        );
    }
    assert_eq!(*starts.last().unwrap(), tz::start_of_local_day_ms(z, TRANSITION));
}

/// The order this whole thing is about: placed at 00:30 local on the day
/// the clocks went back, it belongs to that day and not the one before.
#[test]
fn an_order_just_after_a_long_nights_midnight_is_that_days() {
    let z = default_zone();
    let starts = day_starts(z, TRANSITION, 7);
    // Index 5 is 25 October, the long day; index 6 is today, the 26th.
    let half_past = starts[5] + 30 * 60_000;
    assert_eq!(
        bucket(&starts, half_past, TRANSITION),
        Some(5),
        "00:30 local on the 25th is the 25th; the nominal-day window said the 24th"
    );
}

/// AN ORDER DATED IN THE FUTURE IS NOT TODAY'S TAKINGS. The shipped fold
/// clamped every out-of-range index into the last bucket, so one envelope
/// with a skewed clock added itself to today's revenue with no number
/// anywhere to disagree.
#[test]
fn an_order_dated_in_the_future_is_outside_the_window() {
    let z = default_zone();
    let starts = day_starts(z, WINTER, 7);
    assert_eq!(bucket(&starts, WINTER, WINTER), Some(6), "now is today");
    assert_eq!(
        bucket(&starts, WINTER + DAY_MS, WINTER),
        None,
        "tomorrow's order is not today's takings"
    );
    assert_eq!(bucket(&starts, starts[0] - 1, WINTER), None, "older than the window");
    assert_eq!(bucket(&starts, starts[0], WINTER), Some(0), "the oldest boundary is in");
}

/// THE VENUE'S HOUR, in both halves of the year. 20:00 local in July and
/// 20:00 local in December are the same hour of the venue's day, and were
/// two different hours to the constant.
#[test]
fn the_hour_of_the_day_is_the_venues_hour_all_year() {
    let z = default_zone();
    // 19:00 UTC is 21:00 local in July (+2) and 20:00 local in December (+1).
    assert_eq!(local_hour(z, SUMMER + 7 * 3_600_000), 21);
    assert_eq!(
        local_hour(z, WINTER + 7 * 3_600_000),
        20,
        "the constant made this 21 and put a December dinner in the wrong hour"
    );
}

/// A NAMELESS ROW CARRYING THE REVENUE OF EVERY DAMAGED LINE.
#[test]
fn a_line_with_an_empty_product_id_is_not_a_dish() {
    assert_eq!(dish_id(&json!({ "product_id": "p1" })), Some("p1"));
    assert_eq!(dish_id(&json!({ "quantity": 2 })), None, "absent is not a dish");
    assert_eq!(dish_id(&json!({ "product_id": "" })), None, "and neither is empty");
    assert_eq!(dish_id(&json!({ "product_id": 7 })), None, "nor a number");
}

/// A WEEK HAS SEVEN DAYS, or every day after the gap shifts onto the wrong
/// weekday.
#[test]
fn a_week_is_exactly_seven_days() {
    assert!(week_ok(&json!([[], [], [], [], [], [], []])).is_ok(), "a closed week is a week");
    for wrong in [json!([]), json!([[], [], [], [], [], []]), json!([[], [], [], [], [], [], [], []])] {
        let e = week_ok(&wrong).unwrap_err();
        assert!(e.contains("seven days"), "{wrong} was accepted, or refused for the wrong reason: {e}");
    }
    assert!(week_ok(&json!("monday")).is_err(), "a string is not a week");
}

/// A WINDOW THAT CLOSES BEFORE IT OPENS IS A LATE-NIGHT WINDOW, and the
/// test asks the hub whether it really covers after midnight rather than
/// restating the belief.
#[test]
fn a_window_that_wraps_past_midnight_is_accepted() {
    let w = json!([[], [], [], [], [{ "open": 1320, "close": 120 }], [], []]);
    assert!(week_ok(&w).is_ok(), "22:00 -> 02:00 is a late kitchen, not a typo");
    let sched = dowiz_hub::hours::from_json(&w.to_string());
    assert!(sched.is_open_at(4, 1380), "Friday 23:00");
    assert!(sched.is_open_at(5, 60), "Saturday 01:00, on Friday's window");
    assert!(!sched.is_open_at(5, 180), "Saturday 03:00 is shut");
}

/// The window rules are the hub's, and a window it would drop is refused
/// here rather than silently losing a day the owner had just filled in.
#[test]
fn a_window_the_schedule_could_not_read_is_refused() {
    for bad in [
        json!([[{ "open": 600, "close": 600 }], [], [], [], [], [], []]),
        json!([[{ "open": 1440, "close": 60 }], [], [], [], [], [], []]),
        json!([[{ "open": 60, "close": 1441 }], [], [], [], [], [], []]),
        json!([[{ "open": -1, "close": 60 }], [], [], [], [], [], []]),
    ] {
        assert!(week_ok(&bad).is_err(), "{bad} was accepted");
    }
    assert!(
        week_ok(&json!([[{ "open": 0, "close": 1440 }], [], [], [], [], [], []])).is_ok(),
        "open all day is a window"
    );
}
