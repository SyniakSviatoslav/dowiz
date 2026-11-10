//! PURE. THE MONTH'S PLUS, MINUS AND NET. Every source is a store the app already keeps:
//!
//!   PLUS   the takings the console already counts, day rows of the hot log and the cube
//!          (`cube::fold_orders` / `cube.rs`, disjoint by construction), `status::venue_took`
//!          of every order: `in_takings` (not refused, not refunded, not waiting), less the
//!          tip. A till sale IS a log order (`ebills/sales.rs`), channel `ebills`: the row's
//!          `till` is that channel's share of `t`, and `orders` is the rest, so no sale is
//!          counted twice however it reached the log.
//!   MINUS  `StockEvent::Received` with a price (`Entry::value`, "what the paper says it
//!          cost"), the month's taxes and other expenses, payroll (`payroll::rules::month_total`).
//!
//! A DELIVERY WITH NO PRICE adds nothing and is counted (`uncosted`), so the screen says
//! so rather than letting a zero look like a free delivery. A DELIVERY WITH NO DATE cannot
//! be placed in a month: it is counted as `undated`, never guessed into one.

use super::{parse_month, show_month, Expense, Kind, HISTORY};
use crate::hubdo::OrderView;
use crate::services::analytics::cube::{self, DayCube};
use crate::services::analytics::kitchen::ColdRows;
use crate::services::ordering::channel::EBILLS;
use dowiz_hub::stock::journal::Journal;
use dowiz_hub::stock::meta::{day_number, day_of_local_ms};
use dowiz_hub::stock::StockEvent;
use dowiz_hub::tz::{local_ms, Zone};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// yyyymm -> (receipts, costed value, receipts with no price).
pub type Receipts = BTreeMap<i64, (i64, i64, i64)>;

/// One month's numbers, by source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Month {
    pub month: i64,
    pub orders: i64,
    pub till: i64,
    pub purchases: i64,
    pub receipts: i64,
    pub uncosted: i64,
    pub tax: i64,
    pub other: i64,
    pub payroll: i64,
}

impl Month {
    pub fn plus(&self) -> i64 {
        self.orders + self.till
    }
    pub fn minus(&self) -> i64 {
        self.purchases + self.tax + self.other + self.payroll
    }
    pub fn net(&self) -> i64 {
        self.plus() - self.minus()
    }
}

/// `n` months ending with `m` (yyyymm), oldest first.
pub fn months_ending(m: i64, n: usize) -> Vec<i64> {
    let index = m / 100 * 12 + (m % 100 - 1);
    (0..n as i64).rev().map(|k| (index - k) / 12 * 100 + (index - k) % 12 + 1).collect()
}

/// The sources added up per month. `rows`: the hot fold and the cube's rows, disjoint.
pub fn build(months: &[i64], rows: &[&BTreeMap<i64, DayCube>], receipts: &Receipts, expenses: &[Expense], payroll: &dyn Fn(&str) -> Result<i64, String>) -> Result<Vec<Month>, String> {
    let mut out: Vec<Month> = months.iter().map(|m| Month { month: *m, ..Month::default() }).collect();
    let idx = |m: i64| out.iter().position(|x| x.month == m);
    let mut sales = vec![(0i64, 0i64); months.len()];
    for r in rows.iter().flat_map(|map| map.values()) {
        if let Some(i) = idx(r.d / 100) {
            let till = r.c.get(EBILLS).map_or(0, |v| v[1]);
            sales[i].0 += r.t - till;
            sales[i].1 += till;
        }
    }
    let mut spent = vec![(0i64, 0i64); months.len()];
    for e in expenses {
        if let Some(i) = idx(e.day / 100) {
            match e.kind {
                Kind::Tax => spent[i].0 += e.amount,
                Kind::Other => spent[i].1 += e.amount,
            }
        }
    }
    for (i, m) in out.iter_mut().enumerate() {
        (m.orders, m.till) = sales[i];
        (m.tax, m.other) = spent[i];
        let (n, value, uncosted) = receipts.get(&m.month).copied().unwrap_or((0, 0, 0));
        (m.receipts, m.purchases, m.uncosted) = (n, value, uncosted);
        m.payroll = payroll(&show_month(m.month))?;
    }
    Ok(out)
}

/// The journal's deliveries from `since_ms`, by the venue's month; the second number
/// counts the ones with no date.
pub fn receipts_of(j: &Journal, zone: Zone, since_ms: i64) -> (Receipts, i64) {
    let mut out = Receipts::new();
    let mut undated = 0;
    for e in &j.entries {
        if !matches!(e.ev, StockEvent::Received { .. }) {
            continue;
        }
        let Some(at) = e.meta.at else {
            undated += 1;
            continue;
        };
        if at < since_ms {
            continue;
        }
        let m = day_of_local_ms(local_ms(zone, at)) / 100;
        let row = out.entry(m).or_insert((0, 0, 0));
        row.0 += 1;
        match e.value {
            Some(v) => row.1 += v,
            None => row.2 += 1,
        }
    }
    (out, undated)
}

fn row(key: &str, amount: i64) -> Value {
    json!({ "key": key, "amount": amount })
}

/// The whole answer of `GET /api/owner/finance`. PURE: the venue's object calls this with
/// the images it holds. `Err` is `(status, why)`.
#[allow(clippy::too_many_arguments)]
pub fn answer(
    listed: Vec<OrderView>,
    cat: &dowiz_hub::catalog::Catalog,
    stock: &dowiz_hub::stock::StockLog,
    expenses: Vec<(String, Expense)>,
    venue: &str,
    now: i64,
    month: Option<&str>,
    cold: ColdRows,
    payroll: &dyn Fn(&str) -> Result<i64, String>,
) -> std::result::Result<Value, (u16, String)> {
    let zone = crate::services::analytics::handler::zone_of(cat);
    let today = day_of_local_ms(local_ms(zone, now));
    let selected = match month.map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => parse_month(s).ok_or((400, format!("{s:?} is not a month (yyyy-mm)")))?,
        None => today / 100,
    };
    let months = months_ending(selected, HISTORY);
    let hot = cube::fold_orders(&crate::services::orders::mine::of_venue(listed, venue), zone, now);
    let (first, last) = (months[0] * 100 + 1, selected * 100 + 31);
    let (cold_rows, unread) = match cold(first, last) {
        Ok(r) => (r, None),
        Err(why) => (BTreeMap::new(), Some(why)),
    };
    let since = crate::services::analytics::history::midnight(zone, day_number(first));
    let journal = stock.journal_since(since).map_err(|e| (500, e.to_string()))?;
    let (receipts, undated) = receipts_of(&journal, zone, since);
    let all: Vec<Expense> = expenses.iter().map(|(_, e)| e.clone()).collect();
    let numbers = build(&months, &[&hot, &cold_rows], &receipts, &all, payroll).map_err(|why| (500, why))?;
    let m = numbers.last().cloned().unwrap_or_default();
    let mut rows: Vec<&(String, Expense)> = expenses.iter().filter(|(_, e)| e.day / 100 == selected).collect();
    rows.sort_by(|a, b| b.1.day.cmp(&a.1.day).then_with(|| b.0.cmp(&a.0)));
    let list: Vec<Value> = rows.iter().map(|(id, e)| json!({ "id": id, "date": e.date, "name": e.name, "amount": e.amount, "kind": e.kind })).collect();
    let mut purchases = row("purchases", m.purchases);
    purchases["receipts"] = json!(m.receipts);
    purchases["uncosted"] = json!(m.uncosted);
    purchases["undated"] = json!(undated);
    Ok(json!({
        "contract": "finance.v1",
        "month": show_month(selected),
        "today": dowiz_hub::stock::meta::show_day(today),
        "currency": crate::services::venue::currency_of(cat),
        "expenses": list,
        "tax": { "total": m.tax },
        "other": { "total": m.other },
        "profit": {
            "plus": { "total": m.plus(), "rows": [row("orders", m.orders), row("till", m.till)] },
            "minus": { "total": m.minus(), "rows": [purchases, row("taxes", m.tax), row("other", m.other), row("payroll", m.payroll)] },
            "net": m.net(),
        },
        "history": numbers.iter().map(|x| json!({ "month": show_month(x.month), "plus": x.plus(), "minus": x.minus(), "net": x.net() })).collect::<Vec<_>>(),
        "archive": { "error": unread },
    }))
}

#[cfg(test)]
mod tests;
