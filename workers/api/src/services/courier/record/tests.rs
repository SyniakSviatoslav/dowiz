//! The tally, at the boundaries that cost money when they are wrong.

use super::tally::*;
use serde_json::{json, Value};

const DAY: i64 = 24 * 60 * 60 * 1000;

fn asg(courier: &str, delivered: Option<i64>, cash: i64) -> Value {
    let mut v = json!({ "courier_id": courier, "cash_collected": cash });
    v["delivered_at_ms"] = delivered.map_or(Value::Null, |x| json!(x));
    v
}

/// ANOTHER COURIER'S WORK IS NOT THIS COURIER'S, and the two mistakes here
/// look identical in the answer: counting somebody else's delivery, and
/// counting nothing at all.
#[test]
fn only_this_couriers_assignments_are_counted() {
    let now = 100 * DAY;
    let rows = vec![
        asg("mine", Some(now), 1500),
        asg("theirs", Some(now), 9900),
        asg("mine", None, 0),
    ];
    let t = tally(&rows, "mine", now - DAY, now);
    assert_eq!(t.today_deliveries, 1);
    assert_eq!(t.today_cash, 1500);
    assert_eq!(t.in_flight, 1);
    // And a courier with nothing at all is zeroes, not an absence.
    assert_eq!(tally(&rows, "nobody", now - DAY, now), Tally::default());
}

/// THE DAY BOUNDARY IS THE CASH BOUNDARY. A delivery at the venue's midnight
/// belongs to the day that is starting; one a millisecond earlier belongs to
/// the day that ended, and the cash goes with it. This is the comparison the
/// UTC-day defect got wrong for every venue east of Greenwich.
#[test]
fn a_delivery_at_the_venues_midnight_belongs_to_the_new_day() {
    let start = 50 * DAY;
    let now = start + 3600_000;
    let rows = vec![
        asg("c", Some(start - 1), 1000),
        asg("c", Some(start), 2000),
        asg("c", Some(start + 1), 500),
    ];
    let t = tally(&rows, "c", start, now);
    assert_eq!(t.today_deliveries, 2, "the one before midnight is yesterday's");
    assert_eq!(t.today_cash, 2500);
}

/// THIRTY DAYS IS A ROLLING WINDOW and its far edge is inclusive, so a
/// delivery exactly thirty days ago still counts. Every one of them counts
/// today's cash too only if it is also after the day boundary — the two
/// windows are independent, which is why they are two comparisons.
#[test]
fn the_thirty_day_window_is_rolling_and_its_edge_is_inclusive() {
    let now = 100 * DAY;
    let rows = vec![
        asg("c", Some(now - THIRTY_DAYS_MS), 100),
        asg("c", Some(now - THIRTY_DAYS_MS - 1), 100),
        asg("c", Some(now), 700),
    ];
    let t = tally(&rows, "c", now, now);
    assert_eq!(t.delivered_30d, 2, "the one a millisecond too old is out");
    assert_eq!(t.today_deliveries, 1);
    assert_eq!(t.today_cash, 700);
}

/// AN UNDELIVERED ASSIGNMENT IS IN FLIGHT whether the field is missing or an
/// explicit null, because it is written when the delivery lands. It is never
/// counted as a delivery of zero cash.
#[test]
fn an_undelivered_assignment_is_in_flight_and_never_a_delivery() {
    let now = 10 * DAY;
    let missing = json!({ "courier_id": "c" });
    let rows = vec![missing, asg("c", None, 4000)];
    let t = tally(&rows, "c", now - DAY, now);
    assert_eq!(t.in_flight, 2);
    assert_eq!(t.today_deliveries, 0);
    assert_eq!(t.today_cash, 0, "cash on an undelivered assignment has not been collected");
}

/// CASH IS OPTIONAL AND ABSENT MEANS ZERO here, because a card order carries
/// no cash field at all — but the DELIVERY still counts. Skipping the row
/// would lose the delivery to keep the arithmetic tidy.
#[test]
fn a_card_delivery_counts_without_adding_cash() {
    let now = 10 * DAY;
    let rows = vec![json!({ "courier_id": "c", "delivered_at_ms": now })];
    let t = tally(&rows, "c", now - DAY, now);
    assert_eq!(t.today_deliveries, 1);
    assert_eq!(t.today_cash, 0);
}
