//! The owner's numbers, pinned. Each test is a number an owner makes a
//! decision with.

use super::fold::*;
use dowiz_hub::tz::Zone;
use serde_json::{json, Value};

fn tirane() -> Zone {
    dowiz_hub::tz::zone("Europe/Tirane").expect("the venue's own zone")
}

/// 2026-09-22 12:00 UTC, comfortably inside a summer day.
const NOON: i64 = 1_790_078_400_000;

fn order(at: i64, total: i64, tip: i64, status: &str, kind: &str) -> Value {
    json!({
        "created_at_ms": at, "total": total, "tip": tip, "status": status,
        "fulfilment": { "kind": kind },
        "items": [{ "product_id": "sake", "quantity": 1, "unit_price": total - tip }]
    })
}

// ── the window ──────────────────────────────────────────────────────────────

/// SEVEN OR THIRTY, and nothing else. A console with a stale or hostile query
/// string gets the useful answer rather than a fold over a number it named.
#[test]
fn the_window_is_seven_days_or_thirty() {
    assert_eq!(window(None), 7);
    assert_eq!(window(Some("7")), 7);
    assert_eq!(window(Some("30")), 30);
    assert_eq!(window(Some("365")), 30, "clamped, not honoured");
    assert_eq!(window(Some("0")), 7);
    assert_eq!(window(Some("-1")), 7);
    assert_eq!(window(Some("all")), 7);
}

// ── the days ────────────────────────────────────────────────────────────────

/// EVERY BOUNDARY IS A LOCAL MIDNIGHT. They were `today - k * 86_400_000`,
/// which is only midnight while the offset does not move.
#[test]
fn every_day_start_is_a_local_midnight_and_they_are_oldest_first() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    assert_eq!(starts.len(), 7);
    for (i, s) in starts.iter().enumerate() {
        assert_eq!(
            dowiz_hub::tz::start_of_local_day_ms(z, *s),
            *s,
            "boundary {i} is not a local midnight"
        );
        if i > 0 {
            assert!(*s > starts[i - 1], "oldest first");
        }
    }
    assert_eq!(*starts.last().unwrap(), dowiz_hub::tz::start_of_local_day_ms(z, NOON));
}

/// THE DAY THE CLOCKS GO BACK IS 25 HOURS LONG. Europe/Tirane changes on the
/// last Sunday of October; a window anchored on today's midnight and stepped
/// by 86_400_000 is an hour off local midnight for every day before it, so an
/// order placed at 00:30 the morning after lands in the previous day.
#[test]
fn a_window_that_spans_the_autumn_change_still_starts_each_day_at_midnight() {
    let z = tirane();
    // 2026-10-28 12:00 UTC — three days after the last Sunday of October.
    let after = 1_793_188_800_000;
    let starts = day_starts(z, after, 7);
    for s in &starts {
        assert_eq!(dowiz_hub::tz::start_of_local_day_ms(z, *s), *s);
    }
    let lengths: Vec<i64> = starts.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(
        lengths.iter().any(|l| *l != 86_400_000),
        "this window must contain the long day, or the test proves nothing: {lengths:?}"
    );
}

// ── the fold ────────────────────────────────────────────────────────────────

/// AN ORDER DATED IN THE FUTURE IS NOT TODAY'S. The old fold clamped every
/// out-of-range bucket index into the last one, so an envelope with a skewed
/// clock added itself to today's takings and nothing disagreed.
#[test]
fn an_order_from_the_future_is_outside_the_window_and_not_clamped_into_today() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [order(NOON + 5 * 86_400_000, 1000, 0, "DELIVERED", "delivery")];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.orders, 0);
    assert_eq!(r.revenue, 0);
    assert_eq!(r.by_day.last().unwrap().orders, 0, "today did not absorb it");
}

/// AN ORDER OLDER THAN THE WINDOW IS OUTSIDE IT TOO, and the window starts at
/// the OLDEST midnight, not at "now minus seven days".
#[test]
fn the_window_begins_at_the_oldest_midnight() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [
        order(starts[0] - 1, 5000, 0, "DELIVERED", "delivery"),
        order(starts[0], 1000, 0, "DELIVERED", "delivery"),
    ];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.orders, 1, "one millisecond before the first midnight is last week");
    assert_eq!(r.by_day[0].orders, 1);
    assert_eq!(r.by_day[0].revenue, 1000);
}

/// THE MEAN IS OVER THE ACCEPTED ORDERS. Dividing the takings by every order
/// including the refused ones understates the average by however often the
/// kitchen says no.
#[test]
fn the_average_divides_the_takings_by_the_accepted_orders_only() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [
        order(NOON, 1000, 0, "DELIVERED", "delivery"),
        order(NOON, 2000, 0, "DELIVERED", "delivery"),
        order(NOON, 9000, 0, "REJECTED", "delivery"),
    ];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.orders, 3, "three people tried");
    assert_eq!(r.rejected, 1);
    assert_eq!(r.revenue, 3000, "the refused one is worth nothing");
    assert_eq!(r.average_order, 1500, "3000 over two, not over three");
}

/// EVERY ORDER REFUSED IS NOT A DIVISION BY ZERO.
#[test]
fn a_day_of_nothing_but_refusals_has_an_average_of_zero() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [order(NOON, 9000, 0, "CANCELLED", "delivery")];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.average_order, 0);
    assert_eq!(r.revenue, 0);
    assert_eq!(r.orders, 1);
}

/// THE MEAN ROUNDS DOWN. Money never becomes a float in this system.
#[test]
fn the_average_is_integer_division() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [
        order(NOON, 1000, 0, "DELIVERED", "delivery"),
        order(NOON, 1001, 0, "DELIVERED", "delivery"),
        order(NOON, 1001, 0, "DELIVERED", "delivery"),
    ];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.average_order, 1000, "3002/3 = 1000.67, and 1000 is the honest half");
}

/// A REFUSED ORDER IS STILL A CUSTOMER WHO TRIED: it counts towards when
/// people order and how they wanted it, and towards nothing else.
#[test]
fn a_refusal_counts_as_a_visit_but_not_as_money_or_as_a_dish() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [order(NOON, 5000, 0, "REJECTED", "pickup")];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.pickup, 1);
    assert_eq!(r.delivery, 0);
    assert_eq!(r.by_hour.iter().sum::<i64>(), 1);
    assert_eq!(r.revenue, 0);
    assert!(r.top_products.is_empty(), "nobody ate it");
}

/// THREE KINDS, AND THE `else` BRANCH USED TO SWALLOW THE THIRD.
///
/// The tally was `if kind == "pickup" { pickup } else { delivery }`, so a room
/// full of table orders read as a delivery business on the owner's own
/// analytics pane — the number they would use to decide whether to keep paying
/// couriers. An unknown kind still counts as a pickup rather than vanishing:
/// it is a customer who tried, and the bucket it lands in is the one that
/// claims the least.
#[test]
fn a_table_order_is_not_a_delivery_and_not_a_pickup() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [
        order(NOON, 5000, 0, "DELIVERED", "delivery"),
        order(NOON, 3000, 0, "PICKED_UP", "pickup"),
        order(NOON, 4000, 0, "PICKED_UP", "dine_in"),
        order(NOON, 2000, 0, "PICKED_UP", "dine_in"),
    ];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.delivery, 1, "one, not three");
    assert_eq!(r.pickup, 1);
    assert_eq!(r.dine_in, 2);
    assert_eq!(r.delivery + r.pickup + r.dine_in, os.len() as i64, "every order lands once");
}

/// A LINE WITH NO PRODUCT IS NOT A DISH. These collected under the empty id
/// and could reach the top eight as a nameless row carrying the revenue of
/// every damaged line in the log.
#[test]
fn a_line_with_no_product_id_is_not_a_dish() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let os = [json!({
        "created_at_ms": NOON, "total": 900, "status": "DELIVERED",
        "fulfilment": { "kind": "delivery" },
        "items": [
            { "quantity": 3, "unit_price": 100 },
            { "product_id": "", "quantity": 4, "unit_price": 100 },
            { "product_id": "sake", "quantity": 2, "unit_price": 100 }
        ]
    })];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.top_products.len(), 1);
    assert_eq!(r.top_products[0].id, "sake");
    assert_eq!(r.top_products[0].quantity, 2);
}

/// EIGHT DISHES, BY MONEY, AND A TIE BREAKS ON THE ID so the pane does not
/// reorder itself between two reads of the same log.
#[test]
fn the_top_is_eight_by_money_with_a_stable_tie() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let items: Vec<Value> = (0..12)
        .map(|i| json!({ "product_id": format!("d{i:02}"), "quantity": 1, "unit_price": 100 }))
        .collect();
    let os = [json!({
        "created_at_ms": NOON, "total": 1200, "status": "DELIVERED",
        "fulfilment": { "kind": "delivery" }, "items": items
    })];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.top_products.len(), 8);
    let ids: Vec<&str> = r.top_products.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, ["d00", "d01", "d02", "d03", "d04", "d05", "d06", "d07"]);
}

/// THE VENUE'S HOUR, NOT UTC'S. Europe/Tirane is an hour or two ahead, so an
/// order at 22:00 UTC belongs to midnight local -- which is also the next day,
/// and `by_day` and `by_hour` must agree about that.
#[test]
fn the_hour_and_the_day_are_both_the_venues_own() {
    let z = tirane();
    let starts = day_starts(z, NOON, 7);
    let midnight_local = *starts.last().unwrap();
    let os = [order(midnight_local + 30 * 60_000, 1000, 0, "DELIVERED", "delivery")];
    let r = fold(&os, z, &starts, NOON);
    assert_eq!(r.by_hour[0], 1, "00:30 local is hour zero");
    assert_eq!(r.by_day.last().unwrap().orders, 1, "and it is today");
}

/// AN EMPTY LOG IS A PANE OF ZEROES WITH THE RIGHT NUMBER OF DAYS IN IT, not
/// an empty chart: an owner who sold nothing this week must see the week.
#[test]
fn an_empty_window_still_has_its_days() {
    let z = tirane();
    let starts = day_starts(z, NOON, 30);
    let r = fold(&[], z, &starts, NOON);
    assert_eq!(r.by_day.len(), 30);
    assert!(r.by_day.iter().all(|d| d.orders == 0 && d.revenue == 0));
    assert_eq!(r.by_hour, [0i64; 24]);
    assert_eq!(r.average_order, 0);
}
