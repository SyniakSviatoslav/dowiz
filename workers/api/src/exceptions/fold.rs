//! THE EXCEPTION ROWS: a pure fold over the order log and the till log.
//!
//! BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23 §2.5 (P1-5). Every row is one
//! signed EVENT that already sits in a chained log — the fold only finds it and
//! lays it out as `{at, order_id, kind, reason, amount, by}`. Nothing is
//! persisted, and nothing is totalled or ordered PER PERSON: `by` is a name on a
//! row, which is what a sales-exception report is, never a column that is
//! summed. The groups below are by kind and reason, and by round.
//!
//! The kinds, exactly §2.5's list plus the refund (a payment reversed):
//!   * `void_after_kitchen` — an `Amended` that removed lines from a round the
//!     kitchen had (`room_rules::stage` = InKitchen at that moment);
//!   * `comp` — a line comped (`adjustments[]`, kind `comp`);
//!   * `late_amendment` — an amendment more than N minutes after the round was
//!     placed (the Placed event's seq is its millisecond clock, `next_seq`);
//!   * `refund` — the order's `refund` record;
//!   * `cash_outside_till` — a cash payment carrying no `till_id`;
//!   * `pay_out` — a `till.pay_out` record;
//!   * `over_short` — a closed till period whose recorded over/short is not
//!     zero, one row per currency.

use crate::command::room_rules::{stage, Stage};
use crate::command::till::{Period, PAY_OUT};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

pub const VOID_AFTER_KITCHEN: &str = "void_after_kitchen";
pub const COMP: &str = "comp";
pub const LATE_AMENDMENT: &str = "late_amendment";
pub const REFUND: &str = "refund";
pub const CASH_OUTSIDE_TILL: &str = "cash_outside_till";
pub const PAY_OUT_KIND: &str = "pay_out";
pub const OVER_SHORT: &str = "over_short";

/// One exception: one signed event. `amount` is the stored integer in
/// `currency` (the order's, or the till pile's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Row {
    pub at: i64,
    pub kind: &'static str,
    pub order_id: Option<String>,
    pub till_id: Option<String>,
    pub reason: Option<String>,
    pub amount: i64,
    pub currency: Option<String>,
    pub by: String,
    /// The ledger transaction a wallet-leg finding is about (`legs.rs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_id: Option<String>,
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}
fn n(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}
fn arr<'a>(v: &'a Value, k: &str) -> &'a [Value] {
    v.get(k).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn row(at: i64, kind: &'static str, order: &Value, id: &str, reason: Option<String>, amount: i64, by: String) -> Row {
    Row { at, kind, order_id: Some(id.to_string()), till_id: None, reason, amount, currency: s(order, "currency"), by, tx_id: None }
}

/// The rows the ORDER log holds. `late_ms`: an amendment later than this
/// after placement is a `late_amendment`.
pub fn order_rows(events: &[dowiz_hub::Event], late_ms: i64) -> Vec<Row> {
    let mut state: HashMap<&str, Value> = HashMap::new();
    let mut placed: HashMap<&str, i64> = HashMap::new();
    let mut out = Vec::new();
    for e in events.iter().filter(|e| e.kind.is_order()) {
        let id = e.order_id.as_str();
        if e.kind == dowiz_hub::EventKind::Placed {
            placed.entry(id).or_insert(e.seq as i64);
        }
        let before = state.remove(id).unwrap_or(Value::Null);
        let after = crate::fold::fold_one(before.clone(), &e.order_json);
        if e.kind == dowiz_hub::EventKind::Amended {
            let was_kitchen = stage(before.get("status").and_then(Value::as_str).unwrap_or("")) == Stage::InKitchen;
            for a in &arr(&after, "amended")[arr(&before, "amended").len().min(arr(&after, "amended").len())..] {
                let (at, by, reason) = (n(a, "at"), s(a, "by").unwrap_or_default(), s(a, "reason"));
                let removed: i64 = arr(a, "ops")
                    .iter()
                    .filter(|op| op.get("op").and_then(Value::as_str) == Some("remove"))
                    .filter_map(|op| op.get("line").and_then(Value::as_u64))
                    .filter_map(|i| arr(&before, "items").get(i as usize))
                    .map(|l| n(l, "unit_price").saturating_mul(n(l, "quantity")))
                    .sum();
                if was_kitchen && removed > 0 {
                    out.push(row(at, VOID_AFTER_KITCHEN, &after, id, reason.clone(), removed, by.clone()));
                }
                if placed.get(id).is_some_and(|p| at - p > late_ms) {
                    out.push(row(at, LATE_AMENDMENT, &after, id, reason, removed, by));
                }
            }
            let old = arr(&before, "adjustments").len().min(arr(&after, "adjustments").len());
            for c in arr(&after, "adjustments")[old..].iter().filter(|c| s(c, "kind").as_deref() == Some("comp")) {
                out.push(row(n(c, "at"), COMP, &after, id, s(c, "reason"), n(c, "amount"), s(c, "by").unwrap_or_default()));
            }
        }
        state.insert(id, after);
    }
    for (id, o) in &state {
        for p in arr(o, "payments") {
            if s(p, "method").as_deref() == Some("cash") && p.get("till_id").is_none() {
                let mut r = row(n(p, "at"), CASH_OUTSIDE_TILL, o, id, None, n(p, "amount"), s(p, "by").unwrap_or_default());
                r.currency = s(p, "currency").or(r.currency);
                out.push(r);
            }
        }
        if let Some(r) = o.get("refund").filter(|r| r.get("by").is_some()) {
            out.push(row(n(r, "at"), REFUND, o, id, s(r, "reason"), n(r, "owed"), s(r, "by").unwrap_or_default()));
        }
    }
    out
}

/// The rows the TILL log holds: every pay-out, and every closed period whose
/// recorded over/short is not zero.
pub fn till_rows(entries: &[dowiz_hub::logimage::Entry], periods: &[Period]) -> Vec<Row> {
    let mut out = Vec::new();
    for e in entries.iter().filter(|e| e.kind == PAY_OUT) {
        let v: Value = serde_json::from_str(&e.json).unwrap_or(Value::Null);
        out.push(Row {
            at: n(&v, "at"), kind: PAY_OUT_KIND, order_id: None, till_id: Some(e.subject.clone()),
            reason: s(&v, "reason"), amount: n(&v, "amount"), currency: s(&v, "currency"),
            by: s(&v, "by").unwrap_or_default(), tx_id: None,
        });
    }
    for p in periods {
        for (cur, d) in p.over_short.iter().flatten().filter(|(_, d)| **d != 0) {
            out.push(Row {
                at: p.closed_at.unwrap_or(0), kind: OVER_SHORT, order_id: None, till_id: Some(p.till_id.clone()),
                reason: None, amount: *d, currency: Some(cur.clone()), by: p.closed_by.clone().unwrap_or_default(), tx_id: None,
            });
        }
    }
    out
}

/// Rows with `from <= at <= to`, oldest first.
pub fn window(mut rows: Vec<Row>, from: i64, to: i64) -> Vec<Row> {
    rows.retain(|r| r.at >= from && r.at <= to);
    rows.sort_by(|a, b| a.at.cmp(&b.at).then(a.kind.cmp(b.kind)));
    rows
}

/// One group: a kind and a reason, how many events, and their amounts per currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Group {
    pub kind: &'static str,
    pub reason: Option<String>,
    pub count: usize,
    pub amount: BTreeMap<String, i64>,
}

/// Grouped by kind and reason — never by the person on the row.
pub fn by_reason(rows: &[Row]) -> Vec<Group> {
    let mut out: Vec<Group> = Vec::new();
    for r in rows {
        let cur = r.currency.clone().unwrap_or_default();
        match out.iter_mut().find(|g| g.kind == r.kind && g.reason == r.reason) {
            Some(g) => {
                g.count += 1;
                *g.amount.entry(cur).or_insert(0) += r.amount;
            }
            None => out.push(Group { kind: r.kind, reason: r.reason.clone(), count: 1, amount: [(cur, r.amount)].into() }),
        }
    }
    out
}

/// Grouped by round: each order id with its rows' kinds, in the order they happened.
pub fn by_round(rows: &[Row]) -> Vec<(String, Vec<&'static str>)> {
    let mut out: Vec<(String, Vec<&'static str>)> = Vec::new();
    for r in rows {
        let Some(id) = &r.order_id else { continue };
        match out.iter_mut().find(|(o, _)| o == id) {
            Some((_, k)) => k.push(r.kind),
            None => out.push((id.clone(), vec![r.kind])),
        }
    }
    out
}
