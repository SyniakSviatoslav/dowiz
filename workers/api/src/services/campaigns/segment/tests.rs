//! The four segments on synthetic rows, each with its positive twin, and the
//! lock that holds for all four: no consent, no match.

use super::*;

fn row(last_at: i64) -> Row {
    Row { key: "k1".into(), name: "A".into(), phone: "+355691234567".into(), orders: 3, spent: 900, last_at }
}

/// 2026-09-24 00:00 UTC is day 20720 (checked against Python's calendar).
const SEP_24: i64 = 20_720 * DAY_MS;
const NOW: Now = Now { utc_ms: SEP_24 + 12 * 3_600_000, local_ms: SEP_24 + 14 * 3_600_000 };
const YES: ConsentState = ConsentState { given: true };
const NO: ConsentState = ConsentState { given: false };

fn card(json: &str) -> Record {
    Record::of_cards([json])
}

#[test]
fn the_calendar_is_the_calendar() {
    assert_eq!(civil(0), (1970, 1, 1));
    assert_eq!(civil(20_720), (2026, 9, 24));
    assert_eq!(civil(11_016), (2000, 2, 29));
    assert_eq!(civil(-1), (1969, 12, 31));
}

#[test]
fn everyone_consented_is_exactly_the_consented() {
    let s = Segment::EveryoneConsented;
    assert!(matches(&s, &row(0), &Record::default(), &YES, NOW));
    assert!(!matches(&s, &row(0), &Record::default(), &NO, NOW));
}

#[test]
fn not_seen_since_counts_whole_days_from_the_last_order() {
    let s = Segment::NotSeenSince { days: 30 };
    let long_ago = row(NOW.utc_ms - 31 * DAY_MS);
    let exactly = row(NOW.utc_ms - 30 * DAY_MS);
    let recent = row(NOW.utc_ms - 29 * DAY_MS);
    assert!(matches(&s, &long_ago, &Record::default(), &YES, NOW));
    assert!(matches(&s, &exactly, &Record::default(), &YES, NOW), "thirty days is 'not seen for 30 days'");
    assert!(!matches(&s, &recent, &Record::default(), &YES, NOW));
    assert!(!matches(&s, &long_ago, &Record::default(), &NO, NOW), "absence is not consent");
}

#[test]
fn tag_matches_the_card_and_nothing_else() {
    let s = Segment::Tag { tag: "regular".into() };
    let tagged = card(r#"{"tags":["family","regular"]}"#);
    let other = card(r#"{"tags":["family"]}"#);
    assert!(matches(&s, &row(0), &tagged, &YES, NOW));
    assert!(!matches(&s, &row(0), &other, &YES, NOW));
    assert!(!matches(&s, &row(0), &Record::default(), &YES, NOW), "no card, no tag");
    assert!(!matches(&s, &row(0), &tagged, &NO, NOW));
}

#[test]
fn a_linked_persons_tags_are_the_union_of_their_cards() {
    let r = Record::of_cards([r#"{"tags":["family"]}"#, r#"{"tags":["regular","family"]}"#]);
    assert_eq!(r.tags, vec!["family".to_string(), "regular".to_string()]);
    assert!(matches(&Segment::Tag { tag: "regular".into() }, &row(0), &r, &YES, NOW));
}

#[test]
fn birthday_this_week_is_today_and_the_six_days_after() {
    let s = Segment::BirthdayThisWeek;
    for (b, want) in [("09-24", true), ("09-30", true), ("10-01", false), ("09-23", false)] {
        let r = card(&format!(r#"{{"birthday_md":"{b}"}}"#));
        assert_eq!(matches(&s, &row(0), &r, &YES, NOW), want, "{b}");
    }
    let today = card(r#"{"birthday_md":"09-24"}"#);
    assert!(!matches(&s, &row(0), &today, &NO, NOW));
    assert!(!matches(&s, &row(0), &card(r#"{"birthday_md":"9-24"}"#), &YES, NOW), "malformed is no birthday");
}

/// THE WEEK IS THE VENUE'S. 23:30 UTC on 30 September is already 1 October in
/// Tirana; a customer born on 1 October is in THIS week there.
#[test]
fn the_week_is_counted_on_the_venues_wall_clock() {
    let s = Segment::BirthdayThisWeek;
    let r = card(r#"{"birthday_md":"10-07"}"#);
    let utc_late = SEP_24 + 6 * DAY_MS + 23 * 3_600_000; // 30 Sep 23:00 UTC
    let local_next_day = Now { utc_ms: utc_late, local_ms: utc_late + 2 * 3_600_000 };
    let utc_only = Now { utc_ms: utc_late, local_ms: utc_late };
    assert!(matches(&s, &row(0), &r, &YES, local_next_day));
    assert!(!matches(&s, &row(0), &r, &YES, utc_only));
}

#[test]
fn a_leap_day_birthday_is_kept_on_the_28th_in_a_common_year() {
    let s = Segment::BirthdayThisWeek;
    let r = card(r#"{"birthday_md":"02-29"}"#);
    // 2027-02-25: the week holds 28 Feb, no 29th.
    let common = Now { utc_ms: 20_874 * DAY_MS, local_ms: 20_874 * DAY_MS };
    assert!(matches(&s, &row(0), &r, &YES, common));
    // 2028-02-26: a leap year, the 29th exists and is in the week.
    let leap_year = Now { utc_ms: 21_240 * DAY_MS, local_ms: 21_240 * DAY_MS };
    assert!(matches(&s, &row(0), &r, &YES, leap_year));
    // 2026-09-24: nowhere near.
    assert!(!matches(&s, &row(0), &r, &YES, NOW));
}

#[test]
fn the_closed_list_is_closed_on_the_wire_and_in_its_values() {
    let ok: Segment = serde_json::from_str(r#"{"kind":"not_seen_since","days":30}"#).unwrap();
    assert_eq!(ok, Segment::NotSeenSince { days: 30 });
    assert!(ok.check().is_ok());
    assert!(serde_json::from_str::<Segment>(r#"{"kind":"top_spenders"}"#).is_err(), "no ranking segment exists");
    assert!(Segment::NotSeenSince { days: 0 }.check().is_err());
    assert!(Segment::NotSeenSince { days: 366 }.check().is_err());
    assert!(Segment::Tag { tag: "gold-member".into() }.check().is_err(), "a tag outside the closed list (a tier would be one) is refused");
    assert!(Segment::Tag { tag: "regular".into() }.check().is_ok());
    let back: Segment = serde_json::from_str(&serde_json::to_string(&Segment::BirthdayThisWeek).unwrap()).unwrap();
    assert_eq!(back, Segment::BirthdayThisWeek);
}
