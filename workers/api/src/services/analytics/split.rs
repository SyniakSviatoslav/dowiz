//! THE KITCHEN / DRINKS SPLIT, ADDED UP (W-DRINKS-KITCHEN). PURE. One
//! accumulator both owner screens drive -- the analytics pane over any
//! period (hot orders plus the cube's archived rows) and the till tab over
//! its thirty days -- so the two cannot disagree about one line.
//!
//! TWO KINDS OF NUMBER, kept apart because they come from different places:
//!   * LINES (`add_line`): quantity and line sales per side, the top items of
//!     each side, and each day's sales per side. A hot order gives its lines
//!     with everything the import kept; an archived day gives only the cube's
//!     `dish -> [portions, sales]`, which `side.rs` reads by id.
//!   * CHECKS (`add_order`): how many ACCEPTED orders had a kitchen line, how
//!     many were drinks only, what each group took and its average check --
//!     the number the owner asked for, since one average hides a 3,000-lek
//!     table behind five 79-lek coffees. A check is only known from a whole
//!     order, so this part covers the hot log's days (`ordersFrom`), as
//!     `history::repeat` does for the same reason.
//!
//! Money is integer minor units; an average is integer division over the
//! group's accepted orders; a share is per mille of the lines' sales.

use super::side::{group, Group, Side, Sides};
use crate::services::orders::status;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const KITCHEN: usize = 0;
const DRINKS: usize = 1;
const OPEN: usize = 2;
const KEYS: [&str; 3] = ["kitchen", "drinks", "unclassified"];
/// How many items each side lists.
pub const TOP: usize = 8;

fn slot(s: Option<Side>) -> usize {
    match s {
        Some(Side::Kitchen) => KITCHEN,
        Some(Side::Drinks) => DRINKS,
        None => OPEN,
    }
}

fn group_slot(g: Group) -> usize {
    match g {
        Group::Kitchen => KITCHEN,
        Group::DrinksOnly => DRINKS,
        Group::Unclassified => OPEN,
    }
}

#[derive(Debug, Default, Clone)]
pub struct Split {
    /// Per side: [quantity, sales].
    lines: [[i64; 2]; 3],
    /// Per side: item key -> (name, quantity, sales).
    top: [BTreeMap<String, (Value, i64, i64)>; 3],
    /// Per group: [accepted orders, taken].
    checks: [[i64; 2]; 3],
    /// day key -> sales per side.
    by_day: BTreeMap<String, [i64; 3]>,
}

impl Split {
    /// One sold line: `key` names the item (a product id, or the till's name
    /// when there is none), `name` is what the screen shows.
    pub fn add_line(&mut self, side: Option<Side>, day: &str, key: &str, name: Value, quantity: i64, sales: i64) {
        let i = slot(side);
        self.lines[i][0] += quantity;
        self.lines[i][1] += sales;
        let e = self.top[i].entry(key.to_string()).or_insert((name, 0, 0));
        e.1 += quantity;
        e.2 += sales;
        self.by_day.entry(day.to_string()).or_default()[i] += sales;
    }

    /// One accepted order's group and what the venue took for it.
    pub fn add_check(&mut self, g: Group, took: i64) {
        let i = group_slot(g);
        self.checks[i][0] += 1;
        self.checks[i][1] += took;
    }

    /// A whole hot order: its lines (when it is in takings) and its check.
    /// `day` is the venue's day of the order; a refused or still-waiting
    /// order adds nothing -- it is not a check and sold no dish.
    pub fn add_order(&mut self, o: &Value, day: &str, sides: &Sides) {
        let st = o.get("status").and_then(Value::as_str).unwrap_or("");
        if !status::in_takings(st) {
            return;
        }
        let took = status::venue_took(o.get("total").and_then(Value::as_i64).unwrap_or(0), o.get("tip").and_then(Value::as_i64).unwrap_or(0), st);
        let mut seen = Vec::new();
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let side = sides.of_item(it);
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(0);
            let money = it.get("unit_price").and_then(Value::as_i64).unwrap_or(0) * q;
            let name = it.get("name").and_then(Value::as_str).filter(|n| !n.is_empty());
            let pid = it.get("product_id").and_then(Value::as_str).filter(|p| !p.is_empty());
            let key = pid.map(str::to_string).unwrap_or_else(|| format!("name:{}", name.unwrap_or("")));
            self.add_line(side, day, &key, json!(name.or(pid).unwrap_or("")), q, money);
            seen.push(side);
        }
        self.add_check(group(seen), took);
    }

    /// Everything, in the shape both screens draw (`analytics-sides.js`).
    /// `days` are the period's day keys in order; `orders_from`/`orders_to`
    /// say which days the checks part covers.
    pub fn json(&self, days: &[String], orders_from: &str, orders_to: &str) -> Value {
        let sold: i64 = self.lines.iter().map(|l| l[1]).sum();
        let side = |i: usize| {
            let mut top: Vec<(&String, &(Value, i64, i64))> = self.top[i].iter().collect();
            top.sort_by(|a, b| b.1 .2.cmp(&a.1 .2).then(b.1 .1.cmp(&a.1 .1)).then(a.0.cmp(b.0)));
            json!({
                "quantity": self.lines[i][0], "sales": self.lines[i][1],
                "sharePm": if sold > 0 { Some(self.lines[i][1] * 1000 / sold) } else { None },
                "top": top.iter().take(TOP).map(|(k, (n, q, s))| json!({ "id": k, "name": n, "quantity": q, "sales": s })).collect::<Vec<_>>(),
            })
        };
        let check = |i: usize| {
            let [n, took] = self.checks[i];
            json!({ "orders": n, "revenue": took, "averageCheck": if n > 0 { took / n } else { 0 } })
        };
        let mut out = json!({
            "sold": sold,
            "orders": { "from": orders_from, "to": orders_to, "kitchen": check(KITCHEN), "drinksOnly": check(DRINKS), "unclassified": check(OPEN) },
            "byDay": days.iter().map(|d| { let v = self.by_day.get(d).copied().unwrap_or_default(); json!({ "day": d, "kitchen": v[0], "drinks": v[1], "unclassified": v[2] }) }).collect::<Vec<_>>(),
        });
        for (i, k) in KEYS.iter().enumerate() {
            out[*k] = side(i);
        }
        out
    }
}

/// THE PANE'S SPLIT OVER A PERIOD (`handler::answer_with`): checks and lines
/// from the hot orders inside it, lines from the cube's archived rows of it
/// (`name` is the catalogue's word for a dish id, or the id). `orders.from`
/// is the first day the hot log still holds, clipped to the period.
pub fn period(hot: &[Value], cold: &BTreeMap<i64, super::cube::DayCube>, zone: dowiz_hub::tz::Zone, s: super::history::Span, now: i64, sides: &Sides, name: &dyn Fn(&str) -> Value) -> Value {
    use dowiz_hub::stock::meta::{day_of_number, show_day};
    let (lo, hi) = s.days();
    let mut sp = Split::default();
    let mut oldest: Option<i64> = None;
    for o in hot {
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if at > now {
            continue;
        }
        let d = super::cube::day_of(zone, at);
        oldest = Some(oldest.map_or(d, |x: i64| x.min(d)));
        if d < lo || d > hi {
            continue;
        }
        sp.add_order(o, &show_day(d), sides);
    }
    for (d, row) in cold.range(lo..=hi) {
        for (id, v) in &row.m {
            let n = name(id);
            let side = sides.of_line(Some(id), n.as_str().unwrap_or(id), None);
            sp.add_line(side, &show_day(*d), id, n, v[0], v[1]);
        }
    }
    let days: Vec<String> = (s.first..=s.last()).map(|n| show_day(day_of_number(n))).collect();
    sp.json(&days, &show_day(oldest.map_or(lo, |o| o.max(lo))), &show_day(hi))
}

#[cfg(test)]
#[path = "split/tests.rs"]
mod tests;
