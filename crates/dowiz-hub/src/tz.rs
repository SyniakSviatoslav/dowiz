//! The venue's local time, as a function of its inputs.
//!
//! WHY THIS FILE EXISTS. Local time was a constant in three places --
//! `owner.rs:620`, `storefront.rs:251`, `extra.rs:27` -- all of them `+2 h`, and
//! one of them labelled "Europe/Tirane, standard time". **Europe/Tirane is UTC+1
//! in winter.** The constant is the SUMMER offset, so from 01:00 UTC on the last
//! Sunday of October to the last Sunday of March the venue's day begins an hour
//! early: the owner's daily takings boundary, the schedule's open/closed
//! decision at the edges of the day, and the daily analytics bucket. Nobody had
//! seen it because the platform had not yet had a winter. The first one starts
//! on Sunday 25 October 2026.
//!
//! NO TIMEZONE DATABASE, and none is needed. A venue sits in one place; what it
//! needs is a standard offset and a rule for when that offset changes. The EU
//! rule is two sentences of arithmetic and it is the same rule for every country
//! this product currently serves. A zone whose rule is not in this file is not
//! supported, and `zone()` says so by returning `None` rather than guessing --
//! the settings route refuses the name on write, which is where a typo should
//! be caught.
//!
//! INTEGER MINUTES, no floats, no date library, no clock. Every function here is
//! a function of its arguments, which is the whole reason the defect above was
//! impossible to test and this is not.

/// Milliseconds in a day. Derived rather than written as a number, so the next
/// person can check it.
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// When a zone's offset changes, and by how much.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dst {
    /// The offset never changes. A fixed-offset venue, or one in a country that
    /// abolished the change.
    None,
    /// The European Union rule, which Albania, Ukraine and every other zone in
    /// this file follow: +60 minutes from 01:00 UTC on the last Sunday of March
    /// to 01:00 UTC on the last Sunday of October. Note that the transition
    /// instants are in UTC and are therefore the SAME instant everywhere in the
    /// union -- that is what makes this arithmetic rather than a table.
    Eu,
}

/// A place, as this product needs to know it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Zone {
    /// Minutes east of UTC when the summer rule is NOT in force.
    pub standard_minutes: i64,
    pub dst: Dst,
}

/// The venue this product was built for. Used when a venue names no zone, which
/// every venue does today because the setting did not exist until now.
pub const DEFAULT: Zone = Zone { standard_minutes: 60, dst: Dst::Eu };

/// The same default, spelled. A client that has to render the zone needs the
/// NAME, and a second literal somewhere else is how the two would drift.
pub const DEFAULT_NAME: &str = "Europe/Tirane";

/// The zones this product supports, by IANA name.
///
/// Deliberately short. A name that is not here returns `None` so the caller can
/// refuse it; a silent fallback to the default is how a venue in Kyiv would
/// quietly keep Albanian hours.
pub fn zone(name: &str) -> Option<Zone> {
    let z = match name {
        "Europe/Tirane" => Zone { standard_minutes: 60, dst: Dst::Eu },
        "Europe/Berlin" | "Europe/Rome" | "Europe/Madrid" | "Europe/Paris" | "Europe/Prague"
        | "Europe/Warsaw" | "Europe/Budapest" | "Europe/Belgrade" | "Europe/Vienna"
        | "Europe/Zagreb" | "Europe/Skopje" | "Europe/Podgorica" | "Europe/Sarajevo" => {
            Zone { standard_minutes: 60, dst: Dst::Eu }
        }
        "Europe/Kyiv" | "Europe/Kiev" | "Europe/Athens" | "Europe/Bucharest"
        | "Europe/Sofia" | "Europe/Helsinki" | "Europe/Riga" | "Europe/Vilnius"
        | "Europe/Tallinn" | "Europe/Chisinau" => Zone { standard_minutes: 120, dst: Dst::Eu },
        "Europe/London" | "Europe/Dublin" | "Europe/Lisbon" => {
            Zone { standard_minutes: 0, dst: Dst::Eu }
        }
        "UTC" | "Etc/UTC" => Zone { standard_minutes: 0, dst: Dst::None },
        _ => return None,
    };
    Some(z)
}

/// Every supported name, for the settings surface to offer and for the gate to
/// assert against. Sorted, because a dropdown that reorders itself is a bug
/// report.
pub const NAMES: &[&str] = &[
    "Etc/UTC",
    "Europe/Athens",
    "Europe/Belgrade",
    "Europe/Berlin",
    "Europe/Bucharest",
    "Europe/Budapest",
    "Europe/Chisinau",
    "Europe/Dublin",
    "Europe/Helsinki",
    "Europe/Kiev",
    "Europe/Kyiv",
    "Europe/Lisbon",
    "Europe/London",
    "Europe/Madrid",
    "Europe/Paris",
    "Europe/Podgorica",
    "Europe/Prague",
    "Europe/Riga",
    "Europe/Rome",
    "Europe/Sarajevo",
    "Europe/Skopje",
    "Europe/Sofia",
    "Europe/Tallinn",
    "Europe/Tirane",
    "Europe/Vienna",
    "Europe/Vilnius",
    "Europe/Warsaw",
    "Europe/Zagreb",
    "UTC",
];

/// The zone a venue's settings describe.
///
/// `tz` wins. `tz_offset_minutes` remains as an explicit FIXED-offset override
/// for a venue that wants one -- it was the only control before this module and
/// removing it would change behaviour for anyone who had set it. Nobody had:
/// the key was read with a default of 120 and written nowhere, which is how the
/// summer offset became every venue's winter.
pub fn from_settings(tz_name: Option<&str>, fixed_minutes: Option<i64>) -> Zone {
    if let Some(z) = tz_name.and_then(zone) {
        return z;
    }
    if let Some(m) = fixed_minutes {
        return Zone { standard_minutes: m, dst: Dst::None };
    }
    DEFAULT
}

/// Floor division. Rust's `/` truncates towards zero, which is the wrong
/// rounding for an instant before 1970 and for any negative offset arithmetic.
/// Everything here would work with `/` today and would be wrong the first time
/// it did not.
fn floor_div(a: i64, b: i64) -> i64 {
    let q = a / b;
    if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    }
}

/// Days since 1970-01-01 for a civil date. Howard Hinnant's `days_from_civil`,
/// which is exact integer arithmetic over the proleptic Gregorian calendar.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = floor_div(y, 400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// The civil year containing a day count. Hinnant's `civil_from_days`, reduced
/// to the year, which is all this module needs.
fn year_from_days(z: i64) -> i64 {
    let z = z + 719468;
    let era = floor_div(z, 146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    y + if m <= 2 { 1 } else { 0 }
}

/// 0 = Sunday. 1970-01-01 was a Thursday, which is the +4.
fn weekday(days: i64) -> i64 {
    (days + 4).rem_euclid(7)
}

/// The last Sunday of a 31-day month, as days since the epoch.
///
/// March and October both have 31 days, which is why this is one function and
/// not a calendar: the weekday of the 31st tells you how far back the Sunday is.
fn last_sunday_of_31(y: i64, m: i64) -> i64 {
    let last = days_from_civil(y, m, 31);
    last - weekday(last)
}

/// The instant the EU summer rule begins in a given year: 01:00 UTC on the last
/// Sunday of March.
fn eu_summer_begins(y: i64) -> i64 {
    last_sunday_of_31(y, 3) * DAY_MS + 60 * 60 * 1000
}

/// The instant it ends: 01:00 UTC on the last Sunday of October.
fn eu_summer_ends(y: i64) -> i64 {
    last_sunday_of_31(y, 10) * DAY_MS + 60 * 60 * 1000
}

/// The zone's offset from UTC, in minutes, at a given instant.
pub fn offset_minutes(z: Zone, utc_ms: i64) -> i64 {
    match z.dst {
        Dst::None => z.standard_minutes,
        Dst::Eu => {
            let y = year_from_days(floor_div(utc_ms, DAY_MS));
            if utc_ms >= eu_summer_begins(y) && utc_ms < eu_summer_ends(y) {
                z.standard_minutes + 60
            } else {
                z.standard_minutes
            }
        }
    }
}

/// Local wall-clock milliseconds: what a clock on the wall of the venue reads,
/// expressed as milliseconds since the local epoch. Only ever an intermediate --
/// never store one.
pub fn local_ms(z: Zone, utc_ms: i64) -> i64 {
    utc_ms + offset_minutes(z, utc_ms) * 60_000
}

/// The two offsets a zone can ever be at. Exhaustive, which is what lets
/// `start_of_local_day_ms` search rather than guess.
fn offsets(z: Zone) -> [i64; 2] {
    match z.dst {
        Dst::None => [z.standard_minutes, z.standard_minutes],
        Dst::Eu => [z.standard_minutes, z.standard_minutes + 60],
    }
}

/// The UTC instant at which the venue's current local day began.
///
/// DEFINED, not computed: the largest instant `b <= utc_ms` whose local time is
/// exactly midnight. Said that way the transition days stop being special cases.
///
/// The obvious implementation -- take the offset now, floor to a local day --
/// is wrong on both transition days, because the offset in force NOW is not the
/// offset that was in force at local midnight. On 25 October 2026 the venue is
/// at +1 from 01:00 UTC, but midnight local happened at 22:00 UTC the previous
/// day, at +2. An owner looking at their takings that morning would lose the
/// first hour of the day.
///
/// A two-pass correction is also wrong, and this module's own year-long
/// simulation is what said so: between 22:00 and 23:00 UTC on the transition day
/// it produced 25 Oct 22:00 UTC, which is local 23:00 and not a midnight at all,
/// giving a "local day" one hour long. So: generate every candidate a plausible
/// offset could produce, KEEP ONLY THE ONES THAT REALLY ARE MIDNIGHT, and take
/// the latest. Two offsets and two days is four candidates; the search is
/// cheaper than the reasoning that would justify skipping it.
pub fn start_of_local_day_ms(z: Zone, utc_ms: i64) -> i64 {
    let mut best = i64::MIN;
    for off_min in offsets(z) {
        let off = off_min * 60_000;
        let day = floor_div(utc_ms + off, DAY_MS) * DAY_MS - off;
        // The day before, too: an offset that is not the one in force now can
        // put its own candidate a day ahead of the true boundary.
        for b in [day, day - DAY_MS] {
            if b > utc_ms {
                continue;
            }
            // The test of truth, and the only one: at `b`, is it midnight?
            if (b + offset_minutes(z, b) * 60_000).rem_euclid(DAY_MS) != 0 {
                continue;
            }
            if b > best {
                best = b;
            }
        }
    }
    // Unreachable for any zone this module can construct -- midnight exists --
    // but a silent i64::MIN would be a date in 292 million BC rather than a
    // failure, and this codebase has paid for that shape before.
    debug_assert!(best != i64::MIN, "no local midnight at or before {utc_ms}");
    if best == i64::MIN {
        let off = offset_minutes(z, utc_ms) * 60_000;
        return floor_div(utc_ms + off, DAY_MS) * DAY_MS - off;
    }
    best
}

/// Weekday and minutes since local midnight, for the schedule.
///
/// **0 = MONDAY**, because that is what `hours::Schedule::is_open_at` indexes
/// with and a weekly schedule read a day out is a kitchen reported closed on
/// the wrong day. The private `weekday` above counts Sunday as zero because its
/// job is finding Sundays; these two conventions living in one file is exactly
/// the kind of thing that ships, so the test below pins this one against
/// `hours::local_now` directly rather than against my reading of it.
pub fn local_weekday_minute(z: Zone, utc_ms: i64) -> (usize, i64) {
    crate::hours::local_now(utc_ms, offset_minutes(z, utc_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIRANE: Zone = Zone { standard_minutes: 60, dst: Dst::Eu };

    fn ms(y: i64, m: i64, d: i64, h: i64, min: i64) -> i64 {
        days_from_civil(y, m, d) * DAY_MS + h * 3_600_000 + min * 60_000
    }

    #[test]
    fn the_epoch_was_a_thursday() {
        assert_eq!(weekday(0), 4);
    }

    #[test]
    fn civil_and_days_round_trip() {
        for (y, m, d) in [(1970, 1, 1), (2000, 2, 29), (2026, 9, 21), (2027, 3, 1), (1969, 12, 31)] {
            assert_eq!(year_from_days(days_from_civil(y, m, d)), y, "{y}-{m}-{d}");
        }
    }

    /// The dates this whole module exists for, checked against the calendar.
    #[test]
    fn the_transitions_of_2026_are_the_ones_the_calendar_has() {
        // 29 March 2026 and 25 October 2026 are both Sundays, and both are the
        // last Sunday of their month.
        assert_eq!(eu_summer_begins(2026), ms(2026, 3, 29, 1, 0));
        assert_eq!(eu_summer_ends(2026), ms(2026, 10, 25, 1, 0));
        assert_eq!(eu_summer_begins(2027), ms(2027, 3, 28, 1, 0));
        assert_eq!(eu_summer_ends(2027), ms(2027, 10, 31, 1, 0));
        // 31 March 2024 was itself a Sunday: the edge the `31 - weekday` form
        // gets wrong if it subtracts a week too many.
        assert_eq!(eu_summer_begins(2024), ms(2024, 3, 31, 1, 0));
    }

    #[test]
    fn tirane_is_plus_one_in_winter_and_plus_two_in_summer() {
        assert_eq!(offset_minutes(TIRANE, ms(2026, 1, 15, 12, 0)), 60);
        assert_eq!(offset_minutes(TIRANE, ms(2026, 7, 15, 12, 0)), 120);
        // THE DEFECT: on this day the old constant said 120 and the truth is 60.
        assert_eq!(offset_minutes(TIRANE, ms(2026, 12, 25, 12, 0)), 60);
    }

    #[test]
    fn the_transition_instants_themselves() {
        let begin = eu_summer_begins(2026);
        assert_eq!(offset_minutes(TIRANE, begin - 1), 60);
        assert_eq!(offset_minutes(TIRANE, begin), 120);
        let end = eu_summer_ends(2026);
        assert_eq!(offset_minutes(TIRANE, end - 1), 120);
        assert_eq!(offset_minutes(TIRANE, end), 60);
    }

    /// The reason `start_of_local_day_ms` has two passes.
    #[test]
    fn the_day_boundary_on_the_autumn_transition_keeps_the_first_hour() {
        // 10:00 local on 25 October 2026, which is 09:00 UTC (offset is +1 by
        // then). Local midnight was at 22:00 UTC on the 24th, at +2.
        let now = ms(2026, 10, 25, 9, 0);
        assert_eq!(start_of_local_day_ms(TIRANE, now), ms(2026, 10, 24, 22, 0));
    }

    #[test]
    fn the_day_boundary_on_the_spring_transition() {
        // 29 March 2026: local midnight at 23:00 UTC on the 28th, at +1.
        let now = ms(2026, 3, 29, 10, 0);
        assert_eq!(start_of_local_day_ms(TIRANE, now), ms(2026, 3, 28, 23, 0));
    }

    #[test]
    fn an_ordinary_winter_day_starts_at_twenty_three_hundred_the_night_before() {
        let now = ms(2026, 12, 25, 14, 30);
        assert_eq!(start_of_local_day_ms(TIRANE, now), ms(2026, 12, 24, 23, 0));
    }

    #[test]
    fn an_ordinary_summer_day_starts_at_twenty_two_hundred_the_night_before() {
        let now = ms(2026, 7, 15, 14, 30);
        assert_eq!(start_of_local_day_ms(TIRANE, now), ms(2026, 7, 14, 22, 0));
    }

    /// The simulation the blueprint asks for: a year at hourly steps, and the
    /// local day boundary moves exactly 365 times -- once per local day, never
    /// twice in an hour and never skipped.
    #[test]
    fn a_year_at_hourly_steps_has_exactly_the_days_it_should() {
        let start = ms(2026, 1, 1, 0, 0);
        let end = ms(2027, 1, 1, 0, 0);
        let mut boundaries = 0;
        let mut last = start_of_local_day_ms(TIRANE, start);
        let mut t = start;
        while t < end {
            let b = start_of_local_day_ms(TIRANE, t);
            assert!(b <= t, "a day boundary in the future at {t}");
            assert!(b >= last, "the day boundary went backwards at {t}");
            if b != last {
                boundaries += 1;
                // Consecutive local days are 23, 24 or 25 hours apart, and the
                // short and long ones happen once each.
                let span = b - last;
                assert!(
                    span == 23 * 3_600_000 || span == DAY_MS || span == 25 * 3_600_000,
                    "a local day of {} ms ending at {b}",
                    span
                );
                last = b;
            }
            t += 3_600_000;
        }
        // The walk starts at 00:00 UTC on 1 January, which is already 01:00
        // LOCAL, so the first local day (1 January) is `last` before anything is
        // counted. The last step is 23:00 UTC on 31 December, which is local
        // midnight on 1 January 2027 -- a change. So the changes counted are
        // 2 January 2026 through 1 January 2027 inclusive: 365 of them, for a
        // year of 365 days. Getting this off by one was my arithmetic, not the
        // code's: the assertion that mattered -- no 1-hour "day" -- is above.
        assert_eq!(boundaries, 365, "one boundary per local day change in 2026");
    }

    #[test]
    fn exactly_two_short_or_long_days_in_a_year() {
        let mut odd = 0;
        let mut last = start_of_local_day_ms(TIRANE, ms(2026, 1, 1, 12, 0));
        let mut d = 1;
        while d < 365 {
            let b = start_of_local_day_ms(TIRANE, ms(2026, 1, 1, 12, 0) + d * DAY_MS);
            if b - last != DAY_MS {
                odd += 1;
            }
            last = b;
            d += 1;
        }
        assert_eq!(odd, 2, "spring forward and autumn back, and nothing else");
    }

    /// The two weekday conventions in this file are not the same convention.
    #[test]
    fn the_schedule_weekday_is_the_one_the_schedule_uses() {
        // 21 September 2026 is a Monday.
        let monday = ms(2026, 9, 21, 12, 0);
        let (wd, minute) = local_weekday_minute(TIRANE, monday);
        assert_eq!(wd, 0, "Monday is zero for the schedule");
        assert_eq!(minute, 14 * 60, "12:00 UTC is 14:00 local in September");
        // And the private helper disagrees on purpose: Sunday is zero there.
        assert_eq!(weekday(floor_div(monday, DAY_MS)), 1, "Monday is one for Sundays");
    }

    /// The substitution that replaced `hours::local_now(now, 120)` must agree
    /// with it whenever the offset really is 120 -- otherwise this change moved
    /// every venue's schedule by a day while fixing its hour.
    #[test]
    fn it_agrees_with_hours_local_now_at_the_same_offset() {
        for d in 0..14 {
            let t = ms(2026, 7, 1, 9, 17) + d * DAY_MS;
            assert_eq!(
                local_weekday_minute(TIRANE, t),
                crate::hours::local_now(t, 120),
                "summer, day {d}"
            );
        }
        for d in 0..14 {
            let t = ms(2026, 12, 1, 9, 17) + d * DAY_MS;
            assert_eq!(
                local_weekday_minute(TIRANE, t),
                crate::hours::local_now(t, 60),
                "winter, day {d}"
            );
            // And the old code would have used 120 here, which is the defect.
            assert_ne!(local_weekday_minute(TIRANE, t).1, crate::hours::local_now(t, 120).1);
        }
    }

    #[test]
    fn a_fixed_offset_zone_never_moves() {
        let z = Zone { standard_minutes: 210, dst: Dst::None };
        assert_eq!(offset_minutes(z, ms(2026, 1, 1, 0, 0)), 210);
        assert_eq!(offset_minutes(z, ms(2026, 7, 1, 0, 0)), 210);
    }

    #[test]
    fn settings_prefer_the_name_then_the_fixed_offset_then_the_default() {
        assert_eq!(from_settings(Some("Europe/Kyiv"), None).standard_minutes, 120);
        assert_eq!(from_settings(Some("Europe/Kyiv"), Some(999)).standard_minutes, 120);
        assert_eq!(from_settings(None, Some(330)), Zone { standard_minutes: 330, dst: Dst::None });
        assert_eq!(from_settings(None, None), DEFAULT);
        // An unsupported name is not silently the default -- it falls through to
        // the fixed offset, and the settings route refuses it on write.
        assert_eq!(from_settings(Some("Mars/Olympus"), Some(60)).dst, Dst::None);
    }

    #[test]
    fn the_default_name_is_the_default_zone() {
        assert_eq!(zone(DEFAULT_NAME), Some(DEFAULT));
    }

    #[test]
    fn every_advertised_name_resolves() {
        for n in NAMES {
            assert!(zone(n).is_some(), "{n} is advertised and does not resolve");
        }
    }

    /// The old behaviour, kept honest: a venue that really is at a fixed +2 gets
    /// exactly what the three constants used to give it, all year.
    #[test]
    fn the_old_constant_is_still_reachable_deliberately() {
        let z = from_settings(None, Some(120));
        assert_eq!(offset_minutes(z, ms(2026, 12, 25, 12, 0)), 120);
    }
}
