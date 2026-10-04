//! The alarm's rules, over nothing but lists and instants.

use super::*;
use crate::ebills::state::{Noted, State, MINUTE, SALES_CLOSED_MS};

const NOW: i64 = 1_790_000_000_000;

fn entry(id: &str, kind: &str, next: i64) -> Entry {
    let mut e = Entry::new(id.into(), kind, "chat".into(), "a new order".into(), NOW - 5_000);
    e.next_at_ms = next;
    e
}

/// AN IDLE VENUE SCHEDULES NOTHING. The whole saving: no entry, no link, no
/// fiscal queue -- no alarm, and a run that finds it so deletes the one it had.
#[test]
fn an_idle_venue_schedules_nothing() {
    let want = next_due(&[outbox_next(&[]), ebills_next(false, &State::default(), NOW, &|_| true, 0), fiscal_next(true, 0, NOW)]);
    assert_eq!(want, None);
    assert_eq!(after_run(want, NOW), Arm::Clear);
    assert_eq!(on_write(None, want, NOW), Arm::Keep, "a write that made nothing due arms nothing");
    assert_eq!(rearm(None, want, NOW), Arm::Keep, "and the nightly finds nothing to re-arm");
}

/// AN ENQUEUED MESSAGE IS DUE AT ONCE -- sooner than the minute cron, which
/// could leave it up to sixty seconds.
#[test]
fn an_enqueued_message_arms_the_alarm_for_now() {
    let want = outbox_next(&[entry("o1/telegram", "telegram", NOW)]);
    assert_eq!(want, Some(NOW));
    assert_eq!(on_write(None, want, NOW), Arm::Set(NOW));
    // An entry already overdue is set for now, never for the past.
    assert_eq!(on_write(None, Some(NOW - 9_000), NOW), Arm::Set(NOW));
}

/// A WRITE BRINGS THE ALARM FORWARD AND NEVER PUSHES IT BACK: a later retry
/// written beside an earlier one must not make the earlier one wait.
#[test]
fn a_write_only_brings_the_alarm_forward() {
    assert_eq!(on_write(Some(NOW + 30_000), Some(NOW + 10_000), NOW), Arm::Set(NOW + 10_000));
    assert_eq!(on_write(Some(NOW + 10_000), Some(NOW + 30_000), NOW), Arm::Keep);
    assert_eq!(on_write(Some(NOW + 10_000), Some(NOW + 10_000), NOW), Arm::Keep);
    assert_eq!(on_write(Some(NOW + 10_000), None, NOW), Arm::Keep, "never cleared by a write");
}

/// A RUN RESCHEDULES ONLY WHILE WORK REMAINS, and never sooner than a minute
/// on: an entry that stays due (no bot token yet) is tried a minute later, as
/// the cron did, not in a loop of alarms.
#[test]
fn a_run_reschedules_while_work_remains_and_not_sooner_than_a_minute() {
    assert_eq!(after_run(Some(NOW - 1), NOW), Arm::Set(NOW + RUN_GAP_MS));
    assert_eq!(after_run(Some(NOW + 600_000), NOW), Arm::Set(NOW + 600_000), "a backed-off retry waits its backoff");
    assert_eq!(after_run(None, NOW), Arm::Clear);
    assert_eq!(RUN_GAP_MS, 60_000);
}

/// A PRINT JOB NEVER WAKES THE VENUE: the printer pulls it. A summary due
/// tomorrow sets tomorrow; the earliest entry wins.
#[test]
fn the_outbox_is_due_at_its_earliest_entry_that_the_drain_acts_on() {
    assert_eq!(outbox_next(&[entry("p", crate::print_rail::KIND, NOW)]), None);
    let es = [
        entry("p", crate::print_rail::KIND, NOW - 50_000),
        entry("d", crate::outbox::digest::KIND, NOW + 86_400_000),
        entry("r", "telegram", NOW + 30_000),
    ];
    assert_eq!(outbox_next(&es), Some(NOW + 30_000));
    assert_eq!(outbox_next(&es[..2]), Some(NOW + 86_400_000));
}

/// THE TILL LINK: nothing while unusable or halted; a minute on while open;
/// a backoff waited out.
#[test]
fn the_till_link_fires_a_minute_on_while_open_and_not_at_all_when_halted() {
    let st = State::default();
    assert_eq!(ebills_next(false, &st, NOW, &|_| true, 0), None, "not configured");
    assert_eq!(ebills_next(true, &State { halted: true, ..State::default() }, NOW, &|_| true, 0), None, "halted");
    assert_eq!(ebills_next(true, &st, NOW, &|_| true, 0), Some(NOW + MINUTE));
    let failing = State {
        failures: 3,
        last_error: Some(Noted { at_ms: NOW, sale_id: 0, why: "down".into() }),
        ..State::default()
    };
    assert_eq!(ebills_next(true, &failing, NOW, &|_| true, 0), Some(NOW + 4 * MINUTE));
    // A failure count with no recorded error has nothing to wait from.
    assert_eq!(ebills_next(true, &State { failures: 3, ..State::default() }, NOW, &|_| true, 0), Some(NOW + MINUTE));
}

/// CLOSED, IT WAITS FOR THE SALES LIST -- but never past the minute it opens,
/// and a backlog or a re-check pass fires at once.
#[test]
fn the_till_link_while_closed_waits_for_the_sales_list_or_the_opening() {
    let st = State { last_sales_ms: NOW - 5 * MINUTE, ..State::default() };
    assert_eq!(ebills_next(true, &st, NOW, &|_| false, 0), Some(NOW + SALES_CLOSED_MS - 5 * MINUTE));
    let opens = NOW + 7 * MINUTE;
    assert_eq!(ebills_next(true, &st, NOW, &|t| t >= opens, 0), Some(opens));
    // A list never read is due a minute on.
    assert_eq!(ebills_next(true, &State::default(), NOW, &|_| false, 0), Some(NOW + MINUTE));
    assert_eq!(ebills_next(true, &State { backlog: true, ..st.clone() }, NOW, &|_| false, 0), Some(NOW + MINUTE));
    let pass = State { recheck_from: 5, recheck_until: 9, ..st.clone() };
    assert_eq!(ebills_next(true, &pass, NOW, &|_| false, 0), Some(NOW + MINUTE));
    let over = State { recheck_from: 10, recheck_until: 9, ..st };
    assert_eq!(ebills_next(true, &over, NOW, &|_| false, 0), Some(NOW + SALES_CLOSED_MS - 5 * MINUTE));
}

/// W-LOOP: OPEN AND QUIET, the link backs off 1 -> 2 -> 5 -> 15 minutes from the last thing new,
/// and is back to a minute once something lands. (The state has read its list before: a list
/// never read is due a minute on, above.)
#[test]
fn the_till_link_while_open_backs_off_from_the_last_thing_new() {
    let st = State { last_sales_ms: NOW - MINUTE, ..State::default() };
    let open = |_: i64| true;
    assert_eq!(ebills_next(true, &st, NOW, &open, NOW), Some(NOW + MINUTE), "something just landed");
    assert_eq!(ebills_next(true, &st, NOW, &open, NOW - MINUTE), Some(NOW + 2 * MINUTE));
    assert_eq!(ebills_next(true, &st, NOW, &open, NOW - 3 * MINUTE), Some(NOW + 5 * MINUTE));
    assert_eq!(ebills_next(true, &st, NOW, &open, NOW - 8 * MINUTE), Some(NOW + 15 * MINUTE));
    assert_eq!(ebills_next(true, &st, NOW, &open, 0), Some(NOW + 15 * MINUTE), "never anything new");
    // The state's own record of a sale counts as much as the floor's.
    let sold = State { last_new_ms: NOW, ..st.clone() };
    assert_eq!(ebills_next(true, &sold, NOW, &open, 0), Some(NOW + MINUTE));
    // A backlog is not quiet.
    assert_eq!(ebills_next(true, &State { backlog: true, ..st }, NOW, &open, 0), Some(NOW + MINUTE));
}

/// FISCAL: a minute on while documents are queued, and never while sending is off.
#[test]
fn fiscal_fires_only_when_enabled_and_queued() {
    assert_eq!(fiscal_next(true, 2, NOW), Some(NOW + MINUTE));
    assert_eq!(fiscal_next(false, 2, NOW), None);
    assert_eq!(fiscal_next(true, 0, NOW), None);
    assert_eq!(next_due(&[None, Some(NOW + 5), Some(NOW + 2)]), Some(NOW + 2));
}

/// THE SAFETY NET, with one named stranded entry: `o-stranded/telegram` is due
/// and the object has no alarm -- the nightly sets one for now. An object that
/// has an alarm is left alone, and one with nothing due is idle.
#[test]
fn the_nightly_rearms_a_stranded_entry_and_only_that() {
    let stranded = [entry("o-stranded/telegram", "telegram", NOW - 3_600_000)];
    let want = outbox_next(&stranded);
    let arm = rearm(None, want, NOW);
    assert_eq!(arm, Arm::Set(NOW));
    assert_eq!(Seen::of(arm, None), Seen::Rearmed);
    let kept = rearm(Some(NOW + 5_000), want, NOW);
    assert_eq!(kept, Arm::Keep);
    assert_eq!(Seen::of(kept, Some(NOW + 5_000)), Seen::Armed);
    assert_eq!(Seen::of(rearm(None, None, NOW), None), Seen::Idle);
}

/// THE 50-SUBREQUEST CAP: at most `cap` sends a drain, oldest first; entries
/// that send nothing are never cut.
#[test]
fn one_drain_attempts_at_most_the_cap_of_sends() {
    let es: Vec<Entry> = (0..45).map(|i| entry(&format!("o{i:02}"), "telegram", NOW)).collect();
    let mut all: Vec<&Entry> = es.iter().collect();
    let print = entry("p", crate::print_rail::KIND, NOW);
    let routed = entry("r", crate::notify::route::ROUTE_KIND, NOW);
    all.push(&print);
    all.push(&routed);
    let kept = within_budget(all, SEND_CAP);
    assert_eq!(kept.len(), SEND_CAP + 2);
    assert_eq!(kept[0].id, "o00");
    assert_eq!(kept[SEND_CAP - 1].id, "o39", "the oldest forty");
    assert!(kept.iter().any(|e| e.id == "p") && kept.iter().any(|e| e.id == "r"));
    assert_eq!(within_budget(es.iter().take(3).collect(), SEND_CAP).len(), 3);
    assert_eq!(SEND_CAP, 40);
}

/// ONE LINE A NIGHT, and a lost alarm is named.
#[test]
fn the_nightly_tally_counts_and_names() {
    let mut t = Tally::default();
    t.add("a", Ok(Seen::Armed));
    t.add("b", Ok(Seen::Idle));
    t.add("c", Ok(Seen::Idle));
    t.add("d", Ok(Seen::Rearmed));
    t.add("e", Err("503".into()));
    assert_eq!(t.line(), "timers: 1 armed, 2 idle, 1 re-armed (lost alarms), 1 unreachable");
    assert_eq!(t.rearmed, vec!["d".to_string()]);
    assert_eq!(t.failed, vec!["e: 503".to_string()]);
    assert_eq!(serde_json::to_string(&Seen::Rearmed).unwrap(), "\"rearmed\"");
}

/// THE IMAGES THAT RE-ARM are exactly the three that hold timed work.
#[test]
fn the_timed_images_are_the_outbox_the_till_link_and_fiscal() {
    assert_eq!(TIMED, ["outbox", "ebills", "fiscal"]);
}
