//! The queue's rules, over nothing but a list of entries.

use super::*;

fn entry(id: &str, queued: i64, tries: u32, next: i64) -> Entry {
    Entry {
        id: id.into(),
        kind: "order_placed".into(),
        text: "a new order".into(),
        to: "chat".into(),
        queued_at_ms: queued,
        tries,
        next_at_ms: next,
        handed_ms: None,
        code: None,
    }
}

/// A SUCCESS REMOVES IT. Anything else and the kitchen is told twice.
#[test]
fn a_sent_message_is_not_kept() {
    assert_eq!(after_attempt(&entry("a", 0, 0, 0), true, 1_000), Verdict::Sent);
    // Even one that had failed before: the attempt that worked is the one that
    // counts, and a history of failures does not make a delivered message
    // undelivered.
    assert_eq!(after_attempt(&entry("a", 0, 4, 0), true, 1_000), Verdict::Sent);
}

/// A FAILURE IS RESCHEDULED, NOT DROPPED. This is the whole defect: the inline
/// await had exactly one attempt and no memory of it.
#[test]
fn a_failure_comes_back_later_rather_than_disappearing() {
    let v = after_attempt(&entry("a", 0, 0, 0), false, 1_000);
    assert_eq!(v, Verdict::Retry { tries: 1, next_at_ms: 1_000 + backoff_ms(1) });
}

/// AND IT GIVES UP EVENTUALLY, LOUDLY. Retrying for ever is a different way to
/// lose a message: nobody reads a queue that never drains.
#[test]
fn it_stops_after_six_attempts_and_says_after_how_many() {
    let almost = after_attempt(&entry("a", 0, MAX_TRIES - 2, 0), false, 0);
    assert!(matches!(almost, Verdict::Retry { .. }), "{almost:?}");
    let last = after_attempt(&entry("a", 0, MAX_TRIES - 1, 0), false, 0);
    assert_eq!(last, Verdict::Abandon { after: MAX_TRIES });
}

/// THE FIRST RETRY IS SHORT AND THE LAST IS LONG. The common failure is a
/// moment's unavailability, and the customer is still looking at their
/// confirmation ten seconds later.
#[test]
fn the_backoff_starts_in_seconds_and_ends_in_minutes() {
    assert_eq!(backoff_ms(1), 10_000);
    assert!(backoff_ms(MAX_TRIES - 1) >= 600_000);
    // MONOTONE. A schedule that goes back down would hammer a rail that is
    // already refusing.
    for t in 1..MAX_TRIES {
        assert!(backoff_ms(t + 1) >= backoff_ms(t), "attempt {t} waits longer than {}", t + 1);
    }
}

/// NOTHING IS ATTEMPTED BEFORE ITS TIME, which is what makes the backoff real
/// rather than decorative.
#[test]
fn an_entry_waiting_for_its_backoff_is_not_due() {
    let es = vec![entry("a", 0, 1, 5_000), entry("b", 0, 0, 0)];
    let d = due(&es, 1_000);
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].id, "b");
    assert_eq!(due(&es, 5_000).len(), 2, "and it is due once its instant arrives");
}

/// OLDEST FIRST. A kitchen told about the second order before the first has its
/// queue in the wrong order on the screen.
#[test]
fn the_oldest_message_goes_first() {
    let es = vec![entry("b", 900, 0, 0), entry("a", 100, 0, 0), entry("c", 500, 0, 0)];
    let ids: Vec<&str> = due(&es, 1_000).iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "c", "b"]);
}

/// A FAILING ENTRY MUST NOT STARVE THE ONES BEHIND IT. It is rescheduled into
/// the future, so the next order becomes the oldest due — which is the reason
/// the retry carries an instant rather than a position in a list.
#[test]
fn a_repeatedly_failing_message_does_not_block_the_queue() {
    let stuck = entry("stuck", 0, 0, 0);
    let Verdict::Retry { next_at_ms, .. } = after_attempt(&stuck, false, 1_000) else {
        panic!("a failure must reschedule");
    };
    let es = vec![entry("stuck", 0, 1, next_at_ms), entry("fresh", 2_000, 0, 2_000)];
    let ids: Vec<&str> = due(&es, 3_000).iter().map(|e| e.id.as_str()).collect();
    assert_eq!(ids, vec!["fresh"], "the stuck one is not due yet and does not hold the queue");
}

/// DEPTH AND AGE ARE TWO DIFFERENT QUESTIONS. Two entries two seconds old is a
/// busy lunchtime; two entries an hour old is a broken integration, and the
/// count alone says the same thing about both.
#[test]
fn the_health_pane_can_tell_a_busy_queue_from_a_broken_one() {
    let busy = depth(&[entry("a", 9_000, 0, 0), entry("b", 9_500, 0, 0)], 10_000);
    assert_eq!(busy.waiting, 2);
    assert!(busy.oldest_ms <= 1_000);
    assert_eq!(busy.failing, 0);

    let broken = depth(&[entry("a", 0, 3, 0), entry("b", 500, 1, 0)], 3_600_000);
    assert_eq!(broken.waiting, 2);
    assert_eq!(broken.oldest_ms, 3_600_000);
    assert_eq!(broken.failing, 2, "both have failed at least once");
}

/// An empty queue reports zero age, not a negative one derived from nothing.
#[test]
fn an_empty_queue_is_not_infinitely_old() {
    assert_eq!(depth(&[], 10_000), Depth { waiting: 0, oldest_ms: 0, failing: 0 });
}

/// THE ID IS THE ORDER PLUS THE KIND, so a command retried by the idempotency
/// layer enqueues ONE message rather than two. Idempotency at the queue as well
/// as at the route: the route's key protects the caller, and this protects the
/// kitchen.
#[test]
fn the_same_event_enqueued_twice_is_one_message() {
    let a = Entry::new("ord_1/order_placed".into(), "order_placed", "c".into(), "x".into(), 5);
    let b = Entry::new("ord_1/order_placed".into(), "order_placed", "c".into(), "x".into(), 9);
    assert_eq!(a.id, b.id, "the second write replaces the first rather than adding to it");
    assert_eq!(a.tries, 0);
    assert_eq!(a.next_at_ms, a.queued_at_ms, "a fresh entry is due at once");
}
