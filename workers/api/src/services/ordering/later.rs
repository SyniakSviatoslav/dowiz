//! PURE. "Order for later": the time a customer chose, checked before the order
//! exists.
//!
//! THE DEFECT THIS CLOSES (N4.4; docs/research/2026-10-02-system-integration-check.md
//! item 4). `checkout.js` has offered "at a later time" since the schedule
//! landed and sent `scheduled_for_ms`; `PlaceIn` had no such field, and
//! `serde_json` dropped it at the door. Every order for 19:00 was placed for
//! NOW, cooked at once, and the customer found a cold meal at the time they had
//! asked for. Nothing refused, nothing logged -- the roadmap row blamed
//! `carry.rs`, which had carried the field faithfully all along.
//!
//! THREE RULES, each a refusal the customer reads in their own language: the
//! time is ahead of now by at least `LEAD_MS`, at most `HORIZON_MS`, and inside
//! the venue's opening hours IN THE VENUE'S ZONE (`dowiz-venue-timezone`: a
//! phone in Kyiv picking 19:00 means Durrës 19:00, and the storefront already
//! sends the venue's wall time). A venue with no schedule is open at every
//! hour, which is the rule `fold::menu_venue::at` applies to "open now".

use serde_json::Value;

/// At least this far ahead. "In five minutes" is an order for now, and a
/// kitchen cannot start a scheduled order in the past.
pub const LEAD_MS: i64 = 15 * 60 * 1000;
/// At most a week ahead: a menu and a price list are not promised further.
pub const HORIZON_MS: i64 = 7 * 24 * 60 * 60 * 1000;

/// Why a chosen time is refused.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Refusal {
    Past,
    TooFar,
    Closed,
}

impl Refusal {
    /// THE SECOND WORD IS A KEY. The storefront reads `scheduled_for: <word>`
    /// and shows the sentence in the customer's language (`checkout.js`
    /// `LATER_ERR`); the English after the dashes is for a caller without one.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Past => "scheduled_for: past -- pick a time at least 15 minutes from now",
            Self::TooFar => "scheduled_for: far -- orders are taken up to 7 days ahead",
            Self::Closed => "scheduled_for: closed -- the venue is shut at that time",
        }
    }
}

/// `None` in, `None` out: an order for now. `Some(ms)`: the instant, accepted
/// only inside the window and the venue's opening hours.
pub fn check(scheduled_for_ms: Option<i64>, now_ms: i64, venue: &Value) -> Result<Option<i64>, Refusal> {
    let Some(at) = scheduled_for_ms else { return Ok(None) };
    if at < now_ms + LEAD_MS {
        return Err(Refusal::Past);
    }
    if at > now_ms + HORIZON_MS {
        return Err(Refusal::TooFar);
    }
    if !open_at(venue, at) {
        return Err(Refusal::Closed);
    }
    Ok(Some(at))
}

/// Is the venue open at `at_ms`, by its own schedule in its own zone? No
/// schedule means open at every hour, as the menu's "open now" reads it.
pub fn open_at(venue: &Value, at_ms: i64) -> bool {
    let sched = venue.get("hours").map(|h| dowiz_hub::hours::from_json(&h.to_string())).unwrap_or_default();
    if sched.is_empty() {
        return true;
    }
    let zone = crate::hubstore::zone_of(Some(venue));
    let (weekday, minute) = dowiz_hub::tz::local_weekday_minute(zone, at_ms);
    sched.is_open_at(weekday, minute)
}

/// Is the order ahead of the kitchen's clock? `true` when it is scheduled and
/// the time has not yet come: the queue, the estimate and the board read it.
pub fn is_ahead(order: &Value, now_ms: i64) -> bool {
    order.get("scheduled_for_ms").and_then(Value::as_i64).is_some_and(|s| s > now_ms)
}

#[cfg(test)]
mod tests;
