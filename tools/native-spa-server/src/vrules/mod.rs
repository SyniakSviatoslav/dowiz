//! PURE. The rules this server shares with the Worker, in one place.
//!
//! WHY A FILE FOR THIS. `workers/api` and this crate serve the same owner,
//! courier and storefront surface over the same `dowiz-hub`. This crate was
//! outside CI until today, so every rule the Worker fixed in the meantime was
//! either re-fixed here or silently wrong here — and the ones below were
//! silently wrong. They are pure functions of their arguments, which is what
//! makes the divergence checkable instead of arguable.
//!
//! THE DURABLE FIX IS NOT THIS FILE. A rule with two implementations is a rule
//! that will be fixed once. Every function here belongs in `crates/dowiz-hub`
//! beside `tz.rs`, with `workers/api/src/services/analytics/fold.rs` and this
//! module as its two callers; until that move lands this file is the near
//! copy, written so the move is a deletion rather than a rewrite.

use dowiz_hub::tz::{self, Zone};
use serde_json::Value;

/// Milliseconds in a nominal day. Derived, not written as a number — and the
/// whole point of `day_starts` below is that a venue's day is NOT always this.
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// The venue's time zone, from its own record.
///
/// THE OLD RULE WAS A SUMMER CONSTANT. `TZ_OFFSET_MINUTES`, defaulting to 120,
/// in four places. Europe/Tirane's standard offset is +60; 120 is its SUMMER
/// offset, so from 01:00 UTC on Sunday 25 October 2026 every one of them is an
/// hour early until the last Sunday of March. An env var cannot express that,
/// because it is one number and a zone is a number and a rule.
///
/// `tz` on the venue record wins, then `tz_offset_minutes` as an explicit
/// FIXED-offset override, then `TZ_OFFSET_MINUTES` — kept because an operator
/// who set it meant it — and then `tz::DEFAULT`, which is Europe/Tirane with
/// the EU rule rather than either of its two offsets.
pub fn venue_zone(loc: Option<&Value>) -> Zone {
    let name = loc.and_then(|r| r.get("tz")).and_then(Value::as_str);
    let fixed = loc.and_then(|r| r.get("tz_offset_minutes")).and_then(Value::as_i64);
    let env = std::env::var("TZ_OFFSET_MINUTES").ok().and_then(|v| v.parse::<i64>().ok());
    tz::from_settings(name, fixed.or(env))
}

/// The `days` local midnights ending with today's, oldest first.
///
/// NOT `today - k * 86_400_000`. A local day is not always 24 hours: Europe/
/// Tirane's last Sunday of October is 25 of them. An anchor minus six nominal
/// days is an hour off local midnight for the rest of the window, and an order
/// placed at 00:30 the morning after the change falls into the day before.
/// Each boundary is asked of the zone separately, from one millisecond before
/// the day it follows — an instant that is inside the previous day whatever
/// length either of them turned out to be.
pub fn day_starts(zone: Zone, now: i64, days: usize) -> Vec<i64> {
    let mut out = Vec::with_capacity(days);
    let mut at = tz::start_of_local_day_ms(zone, now);
    for _ in 0..days {
        out.push(at);
        at = tz::start_of_local_day_ms(zone, at - 1);
    }
    out.reverse();
    out
}

/// Which day of the window `at` belongs to, or `None` if it is outside it.
///
/// AN ORDER DATED IN THE FUTURE IS NOT TODAY'S. The shipped fold clamped every
/// out-of-range index into the last bucket, so one envelope with a skewed clock
/// added itself to today's takings and no number anywhere disagreed.
///
/// The index is the last boundary at or before `at` — a search over at most
/// thirty of them, and the only form that survives a day being 23 or 25 hours.
pub fn bucket(starts: &[i64], at: i64, now: i64) -> Option<usize> {
    if starts.is_empty() || at < starts[0] || at > now {
        return None;
    }
    starts.iter().rposition(|s| *s <= at)
}

/// The hour of the VENUE's day an instant falls in, 0..=23.
///
/// Computed per order, because `at` can be months old and the offset in force
/// THEN is the one that decides which hour of the venue's day it belongs to:
/// 20:00 local in July and 20:00 local in December are the same hour, and were
/// two different hours to the constant.
pub fn local_hour(zone: Zone, at: i64) -> usize {
    (tz::local_ms(zone, at).rem_euclid(DAY_MS) / 3_600_000).clamp(0, 23) as usize
}

/// The dish a line names, or nothing.
///
/// A LINE WITH NO PRODUCT IS NOT A DISH. An empty id collects under the empty
/// string and can reach the top products as a nameless row carrying the
/// revenue of every damaged line in the log.
pub fn dish_id(item: &Value) -> Option<&str> {
    item.get("product_id").and_then(Value::as_str).filter(|s| !s.is_empty())
}

/// A week of opening windows, as the owner sent it.
///
/// A WEEK HAS SEVEN DAYS, or every day after a gap sits on the wrong weekday:
/// six arrays meant for Tuesday-to-Sunday are read as Monday-to-Saturday and
/// the venue opens a day early all week.
///
/// The window rules themselves are `dowiz_hub::hours::from_json`'s, checked by
/// PARSING BACK: a window it drops is one the schedule would not have, and the
/// count is what says so. A window that closes before it opens is a LATE-NIGHT
/// window (`22:00 -> 02:00`) and is accepted — refusing it would close every
/// late kitchen on the platform.
pub fn week_ok(h: &Value) -> Result<(), String> {
    if h.as_array().map(Vec::len) != Some(7) {
        return Err("a week has seven days; send seven arrays of windows, \
                    Monday first, an empty one for a day you are closed"
            .into());
    }
    let declared: usize = h
        .as_array()
        .map(|days| days.iter().filter_map(|d| d.as_array()).map(|d| d.len()).sum())
        .unwrap_or(0);
    let readable: usize = dowiz_hub::hours::from_json(&h.to_string())
        .days
        .iter()
        .map(|d| d.len())
        .sum();
    if declared != readable {
        return Err(format!(
            "{} of {declared} time windows could not be read; each needs `open` and \
             `close` as minutes since midnight, 0-1440, and they must differ",
            declared - readable
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
