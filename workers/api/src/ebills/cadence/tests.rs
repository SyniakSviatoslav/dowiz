//! The till link's polling hours and its quiet backoff, at chosen instants.

use super::*;
use serde_json::json;

const MIN: i64 = 60_000;

/// THE BACKOFF: 1, 2, 5, then 15 minutes apart while nothing new comes in.
#[test]
fn a_quiet_till_is_asked_after_1_2_5_then_every_15_minutes() {
    let mut t = 0;
    let mut gaps = Vec::new();
    for _ in 0..6 {
        let g = quiet_gap_ms(t);
        gaps.push(g / MIN);
        t += g;
    }
    assert_eq!(gaps, vec![1, 2, 5, 15, 15, 15]);
    // Something new resets it: the quiet is measured from the change.
    assert_eq!(quiet_gap_ms(0), MIN);
    // Never-seen-anything (quiet since the epoch) is the longest step, not a minute.
    assert_eq!(quiet_gap_ms(i64::MAX / 2), 15 * MIN);
}

/// NO HOURS ON FILE IS NOT "ALWAYS OPEN": it is the storefront's 11:00-23:00 in the venue's zone,
/// as the bookings read it. Seven filed days are the venue's own.
#[test]
fn a_venue_with_no_hours_polls_in_the_storefront_window_not_around_the_clock() {
    let zone = dowiz_hub::tz::from_settings(None, Some(0));
    let day = 1_790_035_200_000; // 2026-09-22T00:00:00Z, a Tuesday
    let none = schedule(&json!({}));
    assert!(!open_at(&none, zone, day + 3 * 60 * MIN), "03:00 with no hours on file is closed");
    assert!(open_at(&none, zone, day + 12 * 60 * MIN), "12:00 is inside the fallback window");
    assert!(!open_at(&none, zone, day + 23 * 60 * MIN + MIN), "23:01 is closed");
    // Seven closed days filed: closed all day (it was "always open" too: an empty schedule).
    let shut = schedule(&json!({"hours": [[], [], [], [], [], [], []]}));
    assert!(!open_at(&shut, zone, day + 12 * 60 * MIN));
}
