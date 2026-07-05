//! Consent + quiet-hours — Q3 🔴 (`docs/design/rebuild-jobs-s8-council/proposal.md` §4.1, REV-S8-6).
//! "Consent is authoritative": prefs are re-checked AT DISPATCH TIME, never trusted from
//! enqueue time (a customer/owner can opt out between when a job was queued and when a worker
//! picks it up — the sub-ms read→send gap this leaves is irreducible and documented, not a bug,
//! per REV-S8-5a). This module is pure decision logic — no DB/HTTP — so
//! "an opted-out customer is never pushed" and "a category-disabled owner target is never pushed"
//! are plain unit tests, independent of the actual re-fetch (`crate::jobs::dispatch` does the
//! re-fetch under tenant isolation and calls into this module with the result).

use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;

/// The three notification categories this port carries (§4.1): `transactional` is
/// default-on and NEVER suppressed by a category pref (only a hard opt-out can block it);
/// `operational` (`shift.*`) and `quality` (`rating.low_received`) are category-gated by the
/// target's `prefs` jsonb.
///
/// `Category`/[`owner_target_allowed`]/[`in_quiet_hours`] are `#[allow(dead_code)]`: they're the
/// OWNER-target half of `notify.dispatch` (§4.1) — real and tested, but their one real caller
/// needs either the Telegram adapter (blocked, `crate::jobs` module doc) or an owner-push loop
/// this pass didn't wire (only the customer-push path is end-to-end via `crate::jobs::worker`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "owner-target dispatch path not wired this pass — see doc above"
)]
pub enum Category {
    Transactional,
    Operational,
    Quality,
}

impl Category {
    /// `transactional` is never suppressed by a category toggle — only consent (opted out /
    /// target disabled) blocks it. Carries `setCategoryPref`'s intent: the categories that CAN be
    /// toggled off are operational/quality, never the order lifecycle itself.
    #[allow(
        dead_code,
        reason = "owner-target dispatch path not wired this pass — see Category's doc"
    )]
    pub fn is_always_on(self) -> bool {
        matches!(self, Category::Transactional)
    }
}

/// Customer push consent — `customer_devices.opted_in` (§4.1). A customer who opted out must not
/// be pushed, full stop; no category exception (customer push carries no category concept — it's
/// the order-status channel or nothing).
pub fn customer_push_allowed(opted_in: bool) -> bool {
    opted_in
}

/// Owner-target consent — combines the target's lifecycle `status` (`active` vs
/// `pending`/`disabled`/`disconnected` — §4.1/§4.2 Q-TG-CIRCUIT: 401/403 → permanently
/// `disabled`) with the category pref. A `transactional` event on an `active` target always
/// sends; anything else needs BOTH `active` AND the category's own pref to be true.
#[allow(
    dead_code,
    reason = "owner-target dispatch path not wired this pass — see Category's doc"
)]
pub fn owner_target_allowed(
    target_status_active: bool,
    category: Category,
    category_pref_on: bool,
) -> bool {
    if !target_status_active {
        return false;
    }
    category.is_always_on() || category_pref_on
}

/// Quiet-hours (§4.1): the ONE tz-aware evaluation in the whole S8 surface (Q-UTC-CRON — every
/// cron/job timestamp otherwise stays UTC). `start_hour`/`end_hour` are LOCAL hours in
/// `location_tz` (0-23); an overnight window (e.g. 22 -> 8) wraps past midnight, which is why
/// this isn't a plain numeric range check. Returns `true` when `now` falls inside the quiet
/// window (i.e. the notification should be SUPPRESSED, unless the caller's category is
/// always-on — quiet-hours is a courtesy for non-transactional pushes, not applied by this
/// function itself, matching `Category::is_always_on`'s carve-out being the caller's job, not
/// this one's — quiet-hours and category-gating are orthogonal checks the dispatcher ANDs
/// together).
#[allow(
    dead_code,
    reason = "owner-target dispatch path not wired this pass — see Category's doc"
)]
pub fn in_quiet_hours(now: DateTime<Utc>, location_tz: Tz, start_hour: u32, end_hour: u32) -> bool {
    let local_hour = now.with_timezone(&location_tz).hour();
    if start_hour == end_hour {
        // A zero-width or full-day window — treat as "never quiet" (a misconfiguration should
        // not silently suppress every notification for the location).
        return false;
    }
    if start_hour < end_hour {
        local_hour >= start_hour && local_hour < end_hour
    } else {
        // Overnight wrap, e.g. 22 -> 8: quiet from start_hour through midnight, then 0 through
        // end_hour.
        local_hour >= start_hour || local_hour < end_hour
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    // ── consent-opt-out-not-pushed (named test, cutover DoD §12 "VAPID/consent (Q3)") ──

    #[test]
    fn opted_out_customer_is_never_pushed() {
        assert!(!customer_push_allowed(false));
        assert!(customer_push_allowed(true));
    }

    #[test]
    fn transactional_always_sends_on_an_active_target_regardless_of_category_pref() {
        assert!(owner_target_allowed(true, Category::Transactional, false));
        assert!(owner_target_allowed(true, Category::Transactional, true));
    }

    #[test]
    fn operational_and_quality_are_category_gated_on_an_active_target() {
        assert!(owner_target_allowed(true, Category::Operational, true));
        assert!(!owner_target_allowed(true, Category::Operational, false));
        assert!(owner_target_allowed(true, Category::Quality, true));
        assert!(!owner_target_allowed(true, Category::Quality, false));
    }

    #[test]
    fn a_disabled_or_pending_target_is_never_pushed_even_for_transactional() {
        assert!(!owner_target_allowed(false, Category::Transactional, true));
        assert!(!owner_target_allowed(false, Category::Operational, true));
    }

    // ── quiet-hours (tz-aware, §4.1) ──

    #[test]
    fn quiet_hours_overnight_window_wraps_past_midnight() {
        let tz: Tz = "UTC".parse().unwrap();
        // 23:00 UTC is inside a 22->8 quiet window.
        let at_2300 = Utc.with_ymd_and_hms(2026, 7, 4, 23, 0, 0).unwrap();
        assert!(in_quiet_hours(at_2300, tz, 22, 8));
        // 03:00 UTC is also inside (the wrapped half).
        let at_0300 = Utc.with_ymd_and_hms(2026, 7, 4, 3, 0, 0).unwrap();
        assert!(in_quiet_hours(at_0300, tz, 22, 8));
        // 12:00 UTC is outside.
        let at_noon = Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap();
        assert!(!in_quiet_hours(at_noon, tz, 22, 8));
    }

    #[test]
    fn quiet_hours_same_day_window_does_not_wrap() {
        let tz: Tz = "UTC".parse().unwrap();
        let at_10 = Utc.with_ymd_and_hms(2026, 7, 4, 10, 0, 0).unwrap();
        assert!(in_quiet_hours(at_10, tz, 9, 17));
        let at_20 = Utc.with_ymd_and_hms(2026, 7, 4, 20, 0, 0).unwrap();
        assert!(!in_quiet_hours(at_20, tz, 9, 17));
    }

    #[test]
    fn quiet_hours_is_evaluated_in_the_locations_timezone_not_utc() {
        // 23:00 UTC is 08:00 in Tokyo (UTC+9) the next day — well OUTSIDE a 22->8 LOCAL quiet
        // window evaluated in Asia/Tokyo, even though it WOULD be inside if evaluated in UTC.
        // This is the exact property that makes tz-awareness load-bearing, not cosmetic.
        let tokyo: Tz = chrono_tz::Asia::Tokyo;
        let at_2300_utc = Utc.with_ymd_and_hms(2026, 7, 4, 23, 0, 0).unwrap();
        assert!(
            !in_quiet_hours(at_2300_utc, tokyo, 22, 8),
            "23:00 UTC is 08:00 JST — the boundary instant, already outside a 22->8 local window"
        );
        let at_1300_utc = Utc.with_ymd_and_hms(2026, 7, 4, 13, 0, 0).unwrap();
        assert!(
            in_quiet_hours(at_1300_utc, tokyo, 22, 8),
            "13:00 UTC is 22:00 JST — the start of the local quiet window"
        );
    }

    #[test]
    fn quiet_hours_zero_width_window_never_suppresses() {
        let tz: Tz = "UTC".parse().unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 4, 3, 0, 0).unwrap();
        assert!(!in_quiet_hours(now, tz, 5, 5));
    }
}
