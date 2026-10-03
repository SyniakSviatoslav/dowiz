//! PURE. THE OWNER'S NUMBERS OVER ANY PERIOD (W-HIST P2c), folded from day
//! rows (`cube::DayCube`): the archived part from the cube image, the rest
//! from the hot log, added day by day (disjoint, `cube.rs`).
//!
//! The v1 fields (`orders`, `revenue`, `byDay`, `topProducts`...) are the same
//! numbers `fold::fold` gives over the same orders (`history/tests.rs` pins
//! that); everything after them is new in `analytics.owner.v2`: the previous
//! period, the same weekday last week and over four weeks, weekday x hour,
//! best and worst hours, the channel mix with money, each dish's trend, and
//! where every day's number came from (hot log or archive) so the screen can
//! open the records behind it.

use super::cube::{DayCube, DAY_MS};
use dowiz_hub::stock::meta::{day_number, day_of_local_ms, day_of_number, parse_day, show_day};
use dowiz_hub::tz::Zone;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const CONTRACT: &str = "analytics.owner.v2";
/// The longest period one read folds: a year, and a leap day.
pub const MAX_SPAN: i64 = 366;
/// How many dishes carry a trend line.
pub const TREND_DISHES: usize = 20;
/// Weeks the weekday average looks back over.
pub const WEEKS: i64 = 4;

/// A run of the venue's days: the first as a day number, and how many.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub first: i64,
    pub n: i64,
}

impl Span {
    pub fn last(&self) -> i64 {
        self.first + self.n - 1
    }
    /// The same length, immediately before.
    pub fn prev(&self) -> Span {
        Span { first: self.first - self.n, n: self.n }
    }
    /// `yyyymmdd` bounds, for reading the cube.
    pub fn days(&self) -> (i64, i64) {
        (day_of_number(self.first), day_of_number(self.last()))
    }
}

/// The period asked for. `from`/`to` (`yyyy-mm-dd`) name one; else `days`
/// (7, 30, 90 or 365, `fold::window`) ending today. Refused: a date that is
/// not one, `from` after `to`, more than `MAX_SPAN` days.
pub fn span(zone: Zone, now: i64, days: Option<&str>, from: Option<&str>, to: Option<&str>) -> Result<Span, String> {
    let today = day_number(day_of_local_ms(dowiz_hub::tz::local_ms(zone, now)));
    let day = |s: &str| parse_day(s).map(day_number).ok_or(format!("{s:?} is not a date (yyyy-mm-dd)"));
    let n = super::fold::window(days);
    let last = match given(to) {
        Some(s) => day(s)?,
        None => today,
    };
    let first = match given(from) {
        Some(s) => day(s)?,
        None => last - n + 1,
    };
    let len = last - first + 1;
    if len < 1 {
        return Err("from is after to".into());
    }
    if len > MAX_SPAN {
        return Err(format!("at most {MAX_SPAN} days at a time"));
    }
    Ok(Span { first, n: len })
}

/// A query value that says something: trimmed, not empty.
fn given(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

/// Monday 0 .. Sunday 6 of a day number (day 0, 1970-01-01, was a Thursday).
pub fn weekday(num: i64) -> usize {
    (num + 3).rem_euclid(7) as usize
}

/// The local midnight of a day number, asked of the zone at that date's UTC
/// noon (a 25-hour day stays one day), as `kitchen::window` does.
pub fn midnight(zone: Zone, num: i64) -> i64 {
    dowiz_hub::tz::start_of_local_day_ms(zone, num * DAY_MS + DAY_MS / 2)
}

/// Every day of `s`, summed.
pub fn total(rows: &BTreeMap<i64, DayCube>, s: Span) -> DayCube {
    let mut t = DayCube::default();
    for num in s.first..=s.last() {
        if let Some(r) = rows.get(&day_of_number(num)) {
            t.add(r);
        }
    }
    t.src.clear();
    t
}

/// INTEGER DIVISION over the ACCEPTED orders: a refused order is not a check.
pub fn average(t: &DayCube) -> i64 {
    if t.o > t.x {
        t.t / (t.o - t.x)
    } else {
        0
    }
}

/// `part` in ‰ of `whole`; `None` without a whole.
pub fn pm(part: i64, whole: i64) -> Option<i64> {
    (whole != 0).then(|| part * 1000 / whole)
}

fn sums(t: &DayCube) -> Value {
    json!({ "orders": t.o, "revenue": t.t, "rejected": t.x, "averageOrder": average(t) })
}

/// This period against another: both, the difference, and the difference in ‰.
fn against(cur: &DayCube, then: &DayCube, s: Span) -> Value {
    let (from, to) = s.days();
    json!({
        "from": show_day(from), "to": show_day(to),
        "orders": then.o, "revenue": then.t, "rejected": then.x, "averageOrder": average(then),
        "delta": { "orders": cur.o - then.o, "revenue": cur.t - then.t, "averageOrder": average(cur) - average(then) },
        "deltaPm": { "orders": pm(cur.o - then.o, then.o), "revenue": pm(cur.t - then.t, then.t) },
    })
}

/// The last day of the period against the same weekday a week before, and
/// against the average of that weekday over the `WEEKS` before it.
fn weekday_compare(rows: &BTreeMap<i64, DayCube>, s: Span) -> Value {
    let day = |num: i64| rows.get(&day_of_number(num)).cloned().unwrap_or_default();
    let last = day(s.last());
    let week = day(s.last() - 7);
    let (mut o, mut t) = (0, 0);
    for w in 1..=WEEKS {
        let r = day(s.last() - 7 * w);
        o += r.o;
        t += r.t;
    }
    json!({
        "day": show_day(day_of_number(s.last())), "weekday": weekday(s.last()),
        "orders": last.o, "revenue": last.t,
        "lastWeek": { "day": show_day(day_of_number(s.last() - 7)), "orders": week.o, "revenue": week.t },
        "average": { "weeks": WEEKS, "orders": o / WEEKS, "revenue": t / WEEKS },
    })
}

/// Hours with orders, best first by money, then by orders, then by hour.
fn hours(t: &DayCube) -> Value {
    let mut open: Vec<usize> = (0..24).filter(|h| t.h[*h] > 0).collect();
    open.sort_by(|a, b| t.ht[*b].cmp(&t.ht[*a]).then(t.h[*b].cmp(&t.h[*a])).then(a.cmp(b)));
    let row = |h: &usize| json!({ "hour": h, "orders": t.h[*h], "revenue": t.ht[*h] });
    let best: Vec<Value> = open.iter().take(3).map(row).collect();
    let worst: Vec<Value> = open.iter().rev().take(3.min(open.len().saturating_sub(3))).map(row).collect();
    json!({ "best": best, "worst": worst })
}

/// The trend's buckets: one per day up to a month, else one per week from
/// the first day (the last one may be shorter).
fn buckets(s: Span) -> Vec<Span> {
    let step = if s.n <= 31 { 1 } else { 7 };
    (0..s.n).step_by(step as usize).map(|k| Span { first: s.first + k, n: step.min(s.n - k) }).collect()
}

/// Everything the analytics pane shows. `cold` is the cube's rows, `hot`
/// the hot log's day partials; both keyed by `yyyymmdd` and both covering
/// the period, the one before it and the `WEEKS` before its last day.
/// `name` is the catalogue's word for a dish.
pub fn report(zone: Zone, s: Span, cold: &BTreeMap<i64, DayCube>, hot: &BTreeMap<i64, DayCube>, name: &dyn Fn(&str) -> Value) -> Value {
    let mut rows = cold.clone();
    for (d, r) in hot {
        rows.entry(*d).or_insert_with(|| DayCube::new(*d)).add(r);
    }
    let cur = total(&rows, s);
    let prev = total(&rows, s.prev());
    let mut grid = vec![[0i64; 24]; 7];
    let by_day: Vec<Value> = (s.first..=s.last())
        .map(|num| {
            let d = day_of_number(num);
            let r = rows.get(&d).cloned().unwrap_or_default();
            for (h, n) in r.h.iter().enumerate() {
                grid[weekday(num)][h] += n;
            }
            json!({ "at": midnight(zone, num), "day": show_day(d), "orders": r.o, "revenue": r.t,
                    "hot": hot.get(&d).map_or(0, |x| x.o), "archived": cold.get(&d).map_or(0, |x| x.o) })
        })
        .collect();
    let mut dishes: Vec<(&String, &[i64; 4])> = cur.m.iter().collect();
    dishes.sort_by(|a, b| b.1[1].cmp(&a.1[1]).then(a.0.cmp(b.0)));
    let top: Vec<Value> = dishes.iter().take(8).map(|(id, v)| json!({ "id": id, "name": name(id.as_str()), "quantity": v[0], "revenue": v[1] })).collect();
    let bs = buckets(s);
    let bucket_rows: Vec<DayCube> = bs.iter().map(|b| total(&rows, *b)).collect();
    let trend: Vec<Value> = dishes
        .iter()
        .take(TREND_DISHES)
        .map(|(id, v)| {
            let before = prev.m.get(*id).copied().unwrap_or_default();
            json!({ "id": id, "name": name(id.as_str()), "quantity": v[0], "revenue": v[1],
                    "prevQuantity": before[0], "prevRevenue": before[1],
                    "series": bucket_rows.iter().map(|b| b.m.get(*id).map_or(0, |x| x[0])).collect::<Vec<_>>() })
        })
        .collect();
    let mut channels: Vec<(&String, &[i64; 2])> = cur.c.iter().collect();
    channels.sort_by(|a, b| b.1[0].cmp(&a.1[0]).then(a.0.cmp(b.0)));
    let (from, to) = s.days();
    json!({
        "contract": CONTRACT,
        "days": s.n, "from": show_day(from), "to": show_day(to),
        "orders": cur.o, "revenue": cur.t, "rejected": cur.x, "averageOrder": average(&cur),
        "delivery": cur.k[0], "pickup": cur.k[1], "dineIn": cur.k[2],
        "byChannel": cur.c.iter().map(|(c, v)| (c.clone(), json!(v[0]))).collect::<serde_json::Map<_, _>>(),
        "byDay": by_day,
        "byHour": cur.h.to_vec(),
        "topProducts": top,
        "foodSales": cur.f,
        "compare": { "prev": against(&cur, &prev, s.prev()), "weekday": weekday_compare(&rows, s) },
        "byWeekdayHour": grid.iter().map(|r| r.to_vec()).collect::<Vec<_>>(),
        "hourRevenue": cur.ht.to_vec(),
        "hours": hours(&cur),
        "channels": channels.iter().map(|(c, v)| json!({ "channel": c, "orders": v[0], "revenue": v[1], "sharePm": pm(v[0], cur.o) })).collect::<Vec<_>>(),
        "trend": { "buckets": bs.iter().map(|b| { let (f, t) = b.days(); json!({ "from": show_day(f), "to": show_day(t) }) }).collect::<Vec<_>>(), "dishes": trend },
        "previous": sums(&prev),
    })
}

/// REPEAT CUSTOMERS AS COUNTS, NEVER AS PEOPLE (no scoring: nobody is
/// ranked, listed or named here). Over the accepted orders in `s` that carry
/// a phone, of the HOT log only -- the cube keeps no person, so the share is
/// honest about the days it covers (`from`).
pub fn repeat(orders: &[Value], zone: Zone, s: Span) -> Value {
    let (lo, hi) = s.days();
    let mut seen: BTreeMap<String, i64> = BTreeMap::new();
    let mut oldest: Option<i64> = None;
    for o in orders {
        let st = o.get("status").and_then(Value::as_str).unwrap_or("");
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        let d = super::cube::day_of(zone, at);
        oldest = Some(oldest.map_or(d, |x: i64| x.min(d)));
        let phone = o.get("contact").and_then(|c| c.get("phone")).and_then(Value::as_str).unwrap_or("");
        if d < lo || d > hi || !crate::services::orders::status::took_money(st) || !crate::services::customers::roll::names_a_person(phone) {
            continue;
        }
        let digits: String = phone.chars().filter(char::is_ascii_digit).collect();
        let digits = digits.strip_prefix("00").unwrap_or(&digits).to_string();
        *seen.entry(digits).or_default() += 1;
    }
    let orders_n: i64 = seen.values().sum();
    let repeat_customers = seen.values().filter(|n| **n > 1).count() as i64;
    let repeat_orders: i64 = seen.values().filter(|n| **n > 1).sum();
    json!({
        "from": show_day(oldest.map_or(lo, |o| o.max(lo))), "to": show_day(hi),
        "customers": seen.len(), "repeatCustomers": repeat_customers,
        "orders": orders_n, "repeatOrders": repeat_orders, "sharePm": pm(repeat_orders, orders_n),
    })
}

#[cfg(test)]
#[path = "history/tests.rs"]
mod tests;
