//! The three rules of an order for later, each with its positive twin, at a
//! known instant: `T0` = 2023-11-14 22:13:20 UTC, a Tuesday, 23:13 in Tirane
//! (UTC+1 in November -- the winter offset the old constant got wrong).

use super::*;
use serde_json::json;

const T0: i64 = crate::edge::site::T0;
const HOUR: i64 = 60 * 60 * 1000;

/// Tuesday 11:00-23:00 and Wednesday 10:00-23:00, Tirane; the rest shut.
fn tirane_venue() -> Value {
    json!({ "tz": "Europe/Tirane",
        "hours": [[], [{"open": 660, "close": 1380}], [{"open": 600, "close": 1380}], [], [], [], []] })
}

#[test]
fn no_time_is_an_order_for_now() {
    assert_eq!(check(None, T0, &json!({})), Ok(None));
    assert_eq!(check(None, T0, &tirane_venue()), Ok(None));
}

#[test]
fn a_time_in_the_past_or_too_close_is_refused_and_one_past_the_lead_is_kept() {
    let v = json!({});
    assert_eq!(check(Some(T0 - HOUR), T0, &v), Err(Refusal::Past));
    assert_eq!(check(Some(T0), T0, &v), Err(Refusal::Past), "now is not later");
    assert_eq!(check(Some(T0 + LEAD_MS - 1), T0, &v), Err(Refusal::Past));
    assert_eq!(check(Some(T0 + LEAD_MS), T0, &v), Ok(Some(T0 + LEAD_MS)), "the lead itself is accepted");
}

#[test]
fn a_time_past_the_horizon_is_refused_and_the_horizon_itself_is_kept() {
    let v = json!({});
    assert_eq!(check(Some(T0 + HORIZON_MS + 1), T0, &v), Err(Refusal::TooFar));
    assert_eq!(check(Some(T0 + HORIZON_MS), T0, &v), Ok(Some(T0 + HORIZON_MS)));
}

/// THE VENUE'S ZONE, NOT UTC: T0 + 2 h is 00:13 UTC Wednesday, which Tirane
/// reads as 01:13 -- shut, since Tuesday's window closed at 23:00. T0 + 12 h
/// is 10:13 UTC, 11:13 Tirane, inside Wednesday's window.
#[test]
fn the_opening_hours_are_read_in_the_venues_zone() {
    let v = tirane_venue();
    assert_eq!(check(Some(T0 + 2 * HOUR), T0, &v), Err(Refusal::Closed));
    assert_eq!(check(Some(T0 + 12 * HOUR), T0, &v), Ok(Some(T0 + 12 * HOUR)));
    // Thursday is a closing day however the hour reads.
    assert_eq!(check(Some(T0 + 36 * HOUR), T0, &v), Err(Refusal::Closed));
    // THE ZONE IS WHAT DECIDES AT THE EDGE: T0 + 11 h is 09:13 UTC, before
    // Wednesday's 10:00 in a fixed-UTC reading, and 10:13 in Tirane -- open.
    let utc = json!({ "tz_offset_minutes": 0, "hours": v["hours"] });
    assert_eq!(check(Some(T0 + 11 * HOUR), T0, &utc), Err(Refusal::Closed));
    assert_eq!(check(Some(T0 + 11 * HOUR), T0, &v), Ok(Some(T0 + 11 * HOUR)));
}

#[test]
fn a_venue_with_no_schedule_is_open_at_every_hour() {
    let v = json!({ "tz": "Europe/Tirane" });
    assert!(open_at(&v, T0 + 3 * HOUR), "03:13 with no schedule is open");
    assert_eq!(check(Some(T0 + 3 * HOUR), T0, &v), Ok(Some(T0 + 3 * HOUR)));
}

/// The storefront keys off the second word; a change here is a change there.
#[test]
fn every_refusal_names_its_key_first() {
    for (r, key) in [(Refusal::Past, "past"), (Refusal::TooFar, "far"), (Refusal::Closed, "closed")] {
        assert!(r.as_str().starts_with(&format!("scheduled_for: {key} ")), "{}", r.as_str());
    }
}

#[test]
fn ahead_means_scheduled_and_not_yet_due() {
    assert!(is_ahead(&json!({"scheduled_for_ms": T0 + HOUR}), T0));
    assert!(!is_ahead(&json!({"scheduled_for_ms": T0 - HOUR}), T0), "its time has come");
    assert!(!is_ahead(&json!({"scheduled_for_ms": T0}), T0));
    assert!(!is_ahead(&json!({"created_at_ms": T0}), T0), "an order for now");
    assert!(!is_ahead(&json!({"scheduled_for_ms": null}), T0));
}
