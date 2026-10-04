//! WHEN THE TILL LINK IS POLLED (W-LOOP, docs/research/2026-10-03-hub-cost-recomputed.md §1.3).
//!
//! THE DEFECT. `sched.is_empty() || sched.is_open_at(..)` read a venue with NO opening hours on
//! file as ALWAYS OPEN, and an open venue's link was polled every minute. One venue's object
//! woke 1,449 times a day, around the clock, with nothing to read -- 95 % of the account's
//! Durable Object requests. Two rules replace it, both pure:
//!
//! * THE HOURS ARE THE STOREFRONT'S. A venue that filed seven days polls inside them; a venue
//!   that filed none gets the window its storefront and its bookings already use for it,
//!   11:00-23:00 in the venue's zone (`booking::hours::schedule_of`, `store/booking.js`
//!   `FALLBACK_WINDOW`) -- one rule for "when is this venue open", never a third. Outside the
//!   hours the link reads only the sales list, every thirty minutes (`state::SALES_CLOSED_MS`).
//! * INSIDE THE HOURS, A QUIET TILL IS ASKED LESS OFTEN: a minute after something new, then
//!   2, 5 and 15 minutes as the quiet goes on (`quiet_gap_ms`), and back to a minute the moment
//!   a sale lands or a table's running total moves. "Something new" is a time the object
//!   already keeps (`State::last_new_ms`, the floor image's `at_ms`), so a quiet firing writes
//!   nothing at all to know it was quiet.

use dowiz_hub::hours::Schedule;
use serde_json::Value;

use super::state::MINUTE;

/// The gaps of the quiet backoff, in minutes: the next poll after this much quiet.
pub(crate) const QUIET_STEPS_MIN: [i64; 4] = [1, 2, 5, 15];

/// The venue's polling hours: its own seven days, or the storefront's fallback window.
pub(crate) fn schedule(loc: &Value) -> Schedule {
    crate::booking::hours::schedule_of(loc.get("hours"))
}

/// Is the venue open at `t`, on its own wall clock?
pub(crate) fn open_at(sched: &Schedule, zone: dowiz_hub::tz::Zone, t: i64) -> bool {
    let (weekday, minute) = dowiz_hub::tz::local_weekday_minute(zone, t);
    sched.is_open_at(weekday, minute)
}

/// The next gap while open, after `quiet_ms` without anything new: the firings fall at
/// +1, +3, +8, +23, +38 ... minutes after the last change -- 1, 2, 5, then 15 minutes apart.
pub(crate) fn quiet_gap_ms(quiet_ms: i64) -> i64 {
    // The firing that follows a gap of `steps[i]` has been quiet for the sum of the gaps so far.
    let mut waited = 0;
    for step in QUIET_STEPS_MIN {
        if quiet_ms < (waited + step) * MINUTE {
            return step * MINUTE;
        }
        waited += step;
    }
    QUIET_STEPS_MIN[QUIET_STEPS_MIN.len() - 1] * MINUTE
}

#[cfg(test)]
mod tests;
