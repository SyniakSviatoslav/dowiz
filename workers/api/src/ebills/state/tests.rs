//! The cadence, the backoff, the venue's day, and the crosswalk SUGGESTION.

use super::*;

/// 2026-09-23 12:00:00 UTC.
const NOON: i64 = 1_790_164_800_000;
/// Tirana in summer: UTC+2.
const TIRANA: i64 = 2 * 3_600_000;

fn cfg() -> Config {
    Config { enabled: true, pos_id: 1, user: "report@x.al".into(), secret: "s".into() }
}

#[test]
fn nothing_is_due_for_a_venue_that_is_off_or_half_configured() {
    let st = State::default();
    assert_eq!(plan(None, &st, NOON, true, NOON + TIRANA), Plan::default());
    for c in [Config { enabled: false, ..cfg() }, Config { secret: String::new(), ..cfg() }, Config { pos_id: 0, ..cfg() }] {
        assert!(!plan(Some(&c), &st, NOON, true, NOON).enabled, "{c:?}");
    }
    // Its positive twin.
    let p = plan(Some(&cfg()), &st, NOON, true, NOON + TIRANA);
    assert!(p.enabled && p.floor && p.sales && p.reread);
    assert_eq!((p.pos_id, p.user.as_str(), p.secret.as_str()), (1, "report@x.al", "s"));
}

#[test]
fn the_floor_every_minute_while_open_the_sales_every_five() {
    let st = State { last_sales_ms: NOON - 2 * MINUTE, last_reread_ms: NOON - MINUTE, ..State::default() };
    let p = plan(Some(&cfg()), &st, NOON, true, NOON);
    assert!(p.floor && !p.sales && !p.reread, "two minutes after a list, only the floor");
    let st = State { last_sales_ms: NOON - 5 * MINUTE + 3_000, ..st };
    assert!(plan(Some(&cfg()), &st, NOON, true, NOON).sales, "a cron three seconds early still counts");
    // Closed: no floor, and the list every thirty minutes.
    let st = State { last_sales_ms: NOON - 10 * MINUTE, ..st };
    let p = plan(Some(&cfg()), &st, NOON, false, NOON);
    assert!(!p.floor && !p.sales);
    let st = State { last_sales_ms: NOON - 30 * MINUTE, ..st };
    assert!(plan(Some(&cfg()), &st, NOON, false, NOON).sales);
    // A backlog is read at once.
    let st = State { last_sales_ms: NOON - MINUTE, backlog: true, ..st };
    assert!(plan(Some(&cfg()), &st, NOON, true, NOON).sales);
}

#[test]
fn a_failing_link_backs_off_and_a_halted_one_waits_for_the_owner() {
    assert_eq!((backoff_ms(0), backoff_ms(1), backoff_ms(2), backoff_ms(3)), (0, MINUTE, 2 * MINUTE, 4 * MINUTE));
    assert_eq!(backoff_ms(40), 60 * MINUTE, "capped at an hour");
    let err = |at| Some(Noted { at_ms: at, sale_id: 0, why: "401".into() });
    let st = State { failures: 3, last_error: err(NOON - 2 * MINUTE), ..State::default() };
    let p = plan(Some(&cfg()), &st, NOON, true, NOON);
    assert!(p.enabled && !p.floor && !p.sales && !p.reread, "four minutes of backoff, two waited");
    let st = State { last_error: err(NOON - 4 * MINUTE), ..st };
    assert!(plan(Some(&cfg()), &st, NOON, true, NOON).sales, "and after four, it tries again");
    let halted = State { halted: true, ..State::default() };
    let p = plan(Some(&cfg()), &halted, NOON, true, NOON);
    assert!(!p.floor && !p.sales && !p.reread);
}

/// The list window is the VENUE's day: 00:30 in Tirana is already the 24th
/// while UTC still says the 23rd.
#[test]
fn the_window_is_the_venues_calendar_day() {
    let half_past_midnight_tirana = NOON + 12 * 3_600_000 - 90 * MINUTE; // 22:30 UTC
    let p = plan(Some(&cfg()), &State::default(), half_past_midnight_tirana, true, half_past_midnight_tirana + TIRANA);
    assert_eq!((p.today.as_str(), p.yesterday.as_str(), p.week_ago.as_str()), ("2026-09-24", "2026-09-23", "2026-09-17"));
    assert_eq!(super::super::time::day_of(0), "1970-01-01");
    assert_eq!(super::super::time::day_of(951_868_800_000), "2000-03-01");
    assert_eq!(super::super::time::day_of(1_709_251_199_900), "2024-02-29");
}

#[test]
fn names_normalise_and_a_suggestion_is_only_a_suggestion() {
    assert_eq!(norm("Sake Nigiri (2 copë)"), "sake nigiri 2 cope");
    assert_eq!(norm("  KORÇA  "), "korca");
    assert_eq!(norm("J&amp;B"), norm("J&B"), "the catalogue escapes what the till does not");
    let products = vec![
        ("p1".to_string(), "Korça".to_string(), 250),
        ("p2".to_string(), "korca".to_string(), 300),
        ("p3".to_string(), "Birra".to_string(), 250),
    ];
    assert_eq!(suggest("korca", 250, &products), vec![("p1".to_string(), true), ("p2".to_string(), false)]);
    assert!(suggest("Uji", 100, &products).is_empty());
    assert!(suggest("", 0, &products).is_empty(), "an empty name matches nothing");
}

/// A record that is there and unreadable is an ERROR, not "absent": absent
/// would restart the watermark at zero and re-read a week.
#[test]
fn a_corrupt_state_is_an_error_not_a_fresh_start() {
    let mut t = dowiz_hub::table::Table::create(CEILING).unwrap();
    assert_eq!(get::<State>(&t, K_STATE, ONE), Ok(None));
    put(&mut t, K_STATE, ONE, &State { watermark: 8700, ..State::default() }).unwrap();
    assert_eq!(get::<State>(&t, K_STATE, ONE).unwrap().map(|s| s.watermark), Some(8700));
    t.put(K_STATE, ONE, "{not json", &[], &[]).unwrap();
    assert!(get::<State>(&t, K_STATE, ONE).is_err());
}

/// A SALE NOT YET FISCALISED IS READ AGAIN while its bill may still wait
/// (`PENDING_FOR_MS`), a few a firing, only when the sales are due. Its
/// twins: a refusal time cannot undo (a payment word), one older than the
/// window, and a firing with no sales due are not retried. The `why` is the
/// real `MapError` Debug, as `poll.rs` records it.
#[test]
fn a_sale_not_yet_fiscalised_is_read_again_for_two_days() {
    use crate::ebills::MapError;
    let why = |e: MapError| format!("{e:?}");
    let not_yet = MapError::NotFinished { status: "CLOSED".into(), fiscal: "IN_PROGRESS".into(), draft: 0 };
    assert!(transient(&why(MapError::NotFiscalised)) && transient(&why(not_yet.clone())));
    assert!(!transient(&why(MapError::Payment("MULTIPLE".into()))));
    let no = |id: i64, at: i64, e: MapError| Noted { at_ms: at, sale_id: id, why: why(e) };
    let refused = vec![
        no(1, NOON - PENDING_FOR_MS - 1, MapError::NotFiscalised),
        no(2, NOON - MINUTE, MapError::NotFiscalised),
        no(3, NOON - MINUTE, MapError::Payment("MULTIPLE".into())),
        no(4, NOON - MINUTE, not_yet),
    ];
    let st = State { refused, ..State::default() };
    assert_eq!(plan(Some(&cfg()), &st, NOON, true, NOON).retry, vec![2, 4]);
    let quiet = State { last_sales_ms: NOON - MINUTE, last_reread_ms: NOON, ..st.clone() };
    assert!(plan(Some(&cfg()), &quiet, NOON, true, NOON).retry.is_empty(), "no list due, no retry");
    let many = State { refused: (10..20).map(|i| no(i, NOON, MapError::NotFiscalised)).collect(), ..State::default() };
    assert_eq!(plan(Some(&cfg()), &many, NOON, true, NOON).retry.len(), RETRY_BUDGET);
}
