//! PURE. The owner's numbers, folded from the orders.
//!
//! NO ANALYTICS STORE. A second table of pre-aggregated numbers is a second
//! thing that can disagree with the orders, and at one restaurant the fold is
//! a loop over a few hundred envelopes.
//!
//! EVERY NUMBER HERE IS A NUMBER AN OWNER MAKES DECISIONS WITH, and none of
//! them had a test, because the fold lived inside a handler that needs two
//! Durable Objects to run.

use dowiz_hub::tz::Zone;
use serde_json::Value;

const DAY_MS: i64 = 86_400_000;

/// One day of the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    /// LOCAL midnight, as a UTC instant.
    pub at: i64,
    pub orders: i64,
    pub revenue: i64,
}

/// One dish in the week's top eight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dish {
    pub id: String,
    pub quantity: i64,
    pub revenue: i64,
}

/// Everything the analytics pane shows, except the dish names and the
/// currency, which are the catalogue's to supply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub orders: i64,
    pub revenue: i64,
    pub rejected: i64,
    pub average_order: i64,
    pub delivery: i64,
    pub pickup: i64,
    /// Orders placed AT A TABLE. A third field rather than a third meaning for
    /// `pickup`: an owner deciding whether to keep paying couriers needs to see
    /// the room and the counter apart.
    pub dine_in: i64,
    /// Orders by where they came from (`services::ordering::channel`). A word
    /// outside the closed set is counted under "unknown", never filed under
    /// the storefront: an unrecognised source must not look first-party.
    pub by_channel: std::collections::BTreeMap<String, i64>,
    pub by_day: Vec<Day>,
    pub by_hour: [i64; 24],
    pub top_products: Vec<Dish>,
}

/// The window an owner asked for: seven days or thirty, and nothing else.
pub fn window(days: Option<&str>) -> i64 {
    match days.and_then(|d| d.parse::<i64>().ok()) {
        Some(d) if d >= 30 => 30,
        _ => 7,
    }
}

/// The `days` local midnights ending with today's, oldest first.
///
/// NOT `today - k * 86_400_000`. A local day is not always 24 hours: on the
/// last Sunday of October Europe/Tirane has a 25-hour one, so an anchor minus
/// six times a day is an hour off local midnight for the rest of the window,
/// and an order placed at 00:30 on the day after the change falls into the
/// day before. Each boundary is asked for separately, from an instant an hour
/// inside the previous day, so the zone's own rule decides every one of them.
pub fn day_starts(zone: Zone, now: i64, days: i64) -> Vec<i64> {
    let mut out = Vec::with_capacity(days.max(0) as usize);
    let mut at = dowiz_hub::tz::start_of_local_day_ms(zone, now);
    for _ in 0..days {
        out.push(at);
        // One millisecond before this day began is inside the previous day,
        // whatever length either of them turned out to be.
        at = dowiz_hub::tz::start_of_local_day_ms(zone, at - 1);
    }
    out.reverse();
    out
}

/// Which day of the window `at` belongs to, or `None` if it is outside it.
fn bucket(starts: &[i64], at: i64) -> Option<usize> {
    if starts.is_empty() || at < starts[0] {
        return None;
    }
    // The last boundary at or before `at`. Thirty of them at most.
    Some(starts.iter().rposition(|s| *s <= at).unwrap_or(0))
}

/// Fold this venue's orders into the pane.
///
/// `orders` is already this venue's (see `services::orders::mine`), `starts`
/// is `day_starts`, and `now` bounds the window at the top.
pub fn fold(orders: &[Value], zone: Zone, starts: &[i64], now: i64) -> Report {
    use crate::services::orders::status;

    let mut by_day: Vec<Day> =
        starts.iter().map(|at| Day { at: *at, orders: 0, revenue: 0 }).collect();
    let mut by_hour = [0i64; 24];
    let mut products: Vec<Dish> = Vec::new();
    let (mut count, mut revenue, mut rejected) = (0i64, 0i64, 0i64);
    let (mut delivery, mut pickup, mut dine_in) = (0i64, 0i64, 0i64);
    let mut by_channel = std::collections::BTreeMap::<String, i64>::new();

    for o in orders {
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        // AN ORDER DATED IN THE FUTURE IS NOT TODAY'S. The old fold clamped
        // every out-of-range index into the last bucket, so one envelope with
        // a skewed clock added itself to today's takings and there was no
        // number anywhere that disagreed.
        if at > now {
            continue;
        }
        let Some(idx) = bucket(starts, at) else { continue };

        let st = o.get("status").and_then(Value::as_str).unwrap_or("");
        let refused = !status::took_money(st);
        count += 1;
        if refused {
            rejected += 1;
        }
        let took = status::venue_took(
            o.get("total").and_then(Value::as_i64).unwrap_or(0),
            o.get("tip").and_then(Value::as_i64).unwrap_or(0),
            st,
        );
        revenue += took;
        // A REFUSED ORDER IS STILL A CUSTOMER WHO TRIED, so it counts towards
        // when people order and how they wanted it. Only the money and the
        // dishes are withheld.
        // THREE KINDS, AND THE ELSE BRANCH USED TO SWALLOW THE THIRD. Anything
        // that was not the word "pickup" counted as a delivery, so a room full
        // of table orders would have read as a delivery business on the
        // owner's own analytics pane.
        let source = crate::services::ordering::channel::of(o).map_or("unknown", |c| c);
        *by_channel.entry(source.to_string()).or_default() += 1;
        match crate::services::ordering::fulfilment::of(o) {
            "delivery" => delivery += 1,
            "dine_in" => dine_in += 1,
            _ => pickup += 1,
        }
        by_day[idx].orders += 1;
        by_day[idx].revenue += took;
        // The venue's hour, not UTC's and not a constant's. Computed per order
        // because `at` can be months old and the offset in force THEN is the
        // one that decides which hour of the day that order belongs to: 20:00
        // local in July and 20:00 local in December are the same hour of the
        // venue's day and were two different hours to the old `+2`.
        let local_h = ((dowiz_hub::tz::local_ms(zone, at).rem_euclid(DAY_MS)) / 3_600_000)
            .clamp(0, 23) as usize;
        by_hour[local_h] += 1;

        if refused {
            continue;
        }
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            // A LINE WITH NO PRODUCT IS NOT A DISH. These used to collect under
            // the empty id and could reach the top eight as a nameless row
            // carrying the revenue of every damaged line in the log.
            let Some(id) = it.get("product_id").and_then(Value::as_str).filter(|s| !s.is_empty())
            else {
                continue;
            };
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            let money = it.get("unit_price").and_then(Value::as_i64).unwrap_or(0) * q;
            match products.iter_mut().find(|p| p.id == id) {
                Some(p) => {
                    p.quantity += q;
                    p.revenue += money;
                }
                None => products.push(Dish { id: id.to_string(), quantity: q, revenue: money }),
            }
        }
    }
    // By money, then by id, so a tie is stable across reads rather than
    // whichever way the fold happened to land.
    products.sort_by(|a, b| b.revenue.cmp(&a.revenue).then(a.id.cmp(&b.id)));
    products.truncate(8);

    Report {
        orders: count,
        revenue,
        rejected,
        // INTEGER DIVISION over the ACCEPTED orders. Money never becomes a
        // float in this system, and a mean that rounds down by one lek is
        // honest in a way 2649.9999 is not.
        average_order: if count > rejected { revenue / (count - rejected) } else { 0 },
        delivery,
        pickup,
        dine_in,
        by_channel,
        by_day,
        by_hour,
        top_products: products,
    }
}
