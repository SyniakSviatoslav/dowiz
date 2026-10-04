//! PURE. "MOST ORDERED THIS WEEK: N" -- the storefront's badge, and the same
//! number on the owner's analytics pane (W-MR0 row MR4; docs/research/
//! 2026-10-03-dynamic-menu-and-resilience.md §6 row 5; operator decision 5 of
//! 2026-10-03: the owner-typed `popular` tag is renamed "Venue's pick", and a
//! REAL count stands beside it).
//!
//! A BADGE IS A CLAIM THE DATA MUST SUPPORT. So:
//!   * the window is the venue's last SEVEN LOCAL DAYS, today included -- the
//!     same days `history::span(days=7)` gives the analytics pane;
//!   * an order counts only if the venue took it (`status::took_money`: not
//!     rejected, cancelled or refunded), exactly as the pane's dish numbers do;
//!   * a TEST order counts NOWHERE in the badge: its contact name starts with
//!     a live-proof run's `LIVE-` or a browser flow's `FLOWS-`
//!     (tools/live-proof/run.mjs, e2e/flows/lib.mjs). Its portions are
//!     returned beside the count (`test`) so the owner, and the live probe,
//!     can see they were seen and left out;
//!   * the badge shows only from `THRESHOLD` portions, and it shows the number:
//!     "most ordered" over two plates is not a fact worth printing.
//! Nobody is scored: this counts plates of a DISH, never a person, and keeps
//! no name, phone or key (tools/gates/no-scoring.sh).

use dowiz_hub::stock::meta::{day_number, day_of_number, show_day};
use dowiz_hub::tz::Zone;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const CONTRACT: &str = "menu.week-top.v1";
/// Local days in the window, today included.
pub const DAYS: i64 = 7;
/// Portions a dish needs in the window before the storefront says so. GUESS
/// (2026-10-04): one plate a day over a week; the owner pane shows every count.
pub const THRESHOLD: i64 = 7;
/// How many dishes may wear the badge at once: more and it means nothing.
pub const MAX_BADGES: usize = 6;
/// Contact-name prefixes of the TEST orders the probes and flows place.
pub const TEST_PREFIXES: [&str; 2] = ["LIVE-", "FLOWS-"];

/// One dish's week.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Count {
    pub id: String,
    /// Portions of real orders the venue took, in the window.
    pub n: i64,
    /// Portions of TEST orders in the same window, left out of `n`.
    pub test: i64,
}

/// A TEST order: placed by a probe or a browser flow, by its contact name.
pub fn is_test(o: &Value) -> bool {
    let name = o.get("contact").and_then(|c| c.get("name")).and_then(Value::as_str).unwrap_or("");
    TEST_PREFIXES.iter().any(|p| name.trim_start().starts_with(p))
}

/// The window's first and last local day, `yyyymmdd`.
pub fn window(zone: Zone, now: i64) -> (i64, i64) {
    let today = super::cube::day_of(zone, now);
    (day_of_number(day_number(today) - (DAYS - 1)), today)
}

/// Every dish ordered in the window, most portions first, then by id.
pub fn counts(orders: &[Value], zone: Zone, now: i64) -> Vec<Count> {
    use crate::services::orders::status;
    let (lo, hi) = window(zone, now);
    let mut by: BTreeMap<String, (i64, i64)> = BTreeMap::new();
    for o in orders {
        let at = o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        if at > now {
            continue;
        }
        let d = super::cube::day_of(zone, at);
        if d < lo || d > hi {
            continue;
        }
        if !status::took_money(o.get("status").and_then(Value::as_str).unwrap_or("")) {
            continue;
        }
        let test = is_test(o);
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(id) = it.get("product_id").and_then(Value::as_str).filter(|s| !s.is_empty()) else { continue };
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(0).max(0);
            let e = by.entry(id.to_string()).or_default();
            if test {
                e.1 += q;
            } else {
                e.0 += q;
            }
        }
    }
    let mut out: Vec<Count> = by.into_iter().map(|(id, (n, test))| Count { id, n, test }).collect();
    out.sort_by(|a, b| b.n.cmp(&a.n).then(a.id.cmp(&b.id)));
    out
}

/// The dishes that wear the badge: `n >= THRESHOLD`, at most `MAX_BADGES`.
pub fn badges(all: &[Count]) -> Vec<&Count> {
    all.iter().filter(|c| c.n >= THRESHOLD).take(MAX_BADGES).collect()
}

/// The PUBLIC answer: the window and the badged dishes with their counts.
/// No test count and no dish under the threshold leaves the venue's object.
pub fn public(orders: &[Value], zone: Zone, now: i64) -> Value {
    let all = counts(orders, zone, now);
    let (lo, hi) = window(zone, now);
    json!({
        "contract": CONTRACT, "days": DAYS, "from": show_day(lo), "to": show_day(hi), "threshold": THRESHOLD,
        "dishes": badges(&all).iter().map(|c| json!({ "id": c.id, "n": c.n })).collect::<Vec<_>>(),
    })
}

/// The OWNER's answer, on the analytics pane: every dish counted, its test
/// portions, and whether the storefront shows it -- the same `counts`, so the
/// pane and the badge cannot disagree.
pub fn owner(orders: &[Value], zone: Zone, now: i64, name: &dyn Fn(&str) -> Value) -> Value {
    let all = counts(orders, zone, now);
    let shown: Vec<&str> = badges(&all).iter().map(|c| c.id.as_str()).collect();
    let (lo, hi) = window(zone, now);
    json!({
        "contract": CONTRACT, "days": DAYS, "from": show_day(lo), "to": show_day(hi), "threshold": THRESHOLD,
        "dishes": all.iter().map(|c| json!({ "id": c.id, "name": name(&c.id), "n": c.n, "test": c.test, "badge": shown.contains(&c.id.as_str()) })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
#[path = "week_top/tests.rs"]
mod tests;
