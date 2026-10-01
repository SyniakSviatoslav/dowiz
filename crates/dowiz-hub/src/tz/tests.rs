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
