//! May a guest book THIS minute? The venue's hours and the slot grid, checked
//! by the hub, not only by the screen that offers the times.
//!
//! THE DEFECT (audit D29): the storefront offers only the half hours inside
//! the venue's windows (`lib/booking-time.js` `timesOn`), and the hub checked
//! nothing of the kind -- a direct POST booked 03:00 on a closed day, and the
//! venue found out when the request landed in its console. This is the same
//! rule, in the same terms, so the screen and the hub cannot disagree:
//!
//! * the minute is the VENUE's, from its zone NAME at the slot's own instant
//!   (`tz::local_weekday_minute`), never an offset read today;
//! * a time is `open + k * SLOT_STEP_MIN`, inside a window, with
//!   `LAST_SITTING_MIN` before the close (a close at or before the open is a
//!   kitchen that runs past midnight);
//! * the minutes after midnight are not offered, exactly as `timesOn` states;
//! * a venue that never filed hours gets the storefront's own fallback,
//!   11:00-23:00 -- the hub refuses what the screen would not have offered,
//!   and nothing the screen would.
//!
//! PURE: the tests in `hours/tests.rs` run it at chosen instants.

use dowiz_hub::hours::{self, Schedule, Window, DAY};
use dowiz_hub::tz::{self, Zone};
use serde_json::Value;

/// The grid, as `lib/booking-time.js` exports it (`SLOT_STEP_MIN`).
pub(super) const SLOT_STEP_MIN: i64 = 30;
/// The last sitting is this long before the close (`LAST_SITTING_MIN`).
pub(super) const LAST_SITTING_MIN: i64 = 60;
/// `store/booking.js` `FALLBACK_WINDOW`, for a venue with no hours on file.
const FALLBACK: Window = Window { open: 11 * 60, close: 23 * 60 };

/// The schedule the storefront draws from: the venue's seven days when it
/// filed seven, else the fallback window every day.
pub(super) fn schedule_of(hours_field: Option<&Value>) -> Schedule {
    match hours_field {
        Some(h) if h.as_array().is_some_and(|a| a.len() == 7) => hours::from_json(&h.to_string()),
        _ => Schedule { days: std::array::from_fn(|_| vec![FALLBACK]) },
    }
}

/// `None` when a guest may book `slot_min`; else the sentence that says why not.
pub(super) fn slot_verdict(sched: &Schedule, zone: Zone, slot_min: i64) -> Option<String> {
    let (weekday, minute) = tz::local_weekday_minute(zone, slot_min * 60_000);
    let at = format!("{:02}:{:02}", minute / 60, minute % 60);
    let wins = &sched.days[weekday % 7];
    if wins.is_empty() {
        return Some(format!("the venue takes no bookings on that day ({at} was asked)"));
    }
    let fits = wins.iter().any(|w| {
        let close = if w.close <= w.open { w.close + DAY } else { w.close };
        minute >= w.open && minute + LAST_SITTING_MIN <= close && (minute - w.open) % SLOT_STEP_MIN == 0
    });
    if fits {
        return None;
    }
    let offered: Vec<String> = wins
        .iter()
        .map(|w| format!("{:02}:{:02}-{:02}:{:02}", w.open / 60, w.open % 60, (w.close % DAY) / 60, w.close % 60))
        .collect();
    Some(format!(
        "{at} is not a time this venue takes bookings: every {SLOT_STEP_MIN} minutes from the opening, \
         the last {LAST_SITTING_MIN} minutes before the close, within {}",
        offered.join(", ")
    ))
}

#[cfg(test)]
mod tests;
