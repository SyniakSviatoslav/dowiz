//! PURE. The sitting — the check, the tab — as a PROJECTION of the rounds that
//! carry its id (BLUEPRINT-POS-THE-ROOM §2.2).
//!
//! NO IMAGE AND NO EVENT. Every projection here is rebuilt from the log, never
//! a second source of truth, and a `sitting` record beside the orders would be
//! an aggregate whose status can disagree with its own history — the class
//! `253e1ece` closed. Opening a table is placing its first round; the table is
//! occupied while any round of the sitting is live or its bill is unpaid;
//! closing it is the payment that brings Σ paid to the bill.
//!
//! One function computes the bill, and both the waiter's screen (through the
//! Worker) and the `pay` command (inside the object) ask it, so the number a
//! guest is shown and the number a payment is checked against are one number.

use crate::hubdo::OrderView;
use crate::services::orders::status::{is_terminal, took_money};
use serde_json::{json, Value};

/// A round, parsed, with the version a tablet must quote to edit it.
pub struct Round<'a> {
    pub view: &'a OrderView,
    pub order: Value,
}

impl Round<'_> {
    pub fn int(&self, k: &str) -> i64 {
        self.order.get(k).and_then(Value::as_i64).unwrap_or(0)
    }
    pub fn status(&self) -> &str {
        self.order.get("status").and_then(Value::as_str).unwrap_or("")
    }
    /// Does this round's money count toward the bill? A rejected or cancelled
    /// round took no money, and a guest is not asked to pay for it.
    pub fn billed(&self) -> bool {
        took_money(self.status())
    }
}

/// Every round of one sitting, oldest first.
pub fn rounds<'a>(listed: &'a [OrderView], sitting_id: &str) -> Vec<Round<'a>> {
    let mut out: Vec<Round<'a>> = listed
        .iter()
        .filter_map(|v| {
            let order: Value = serde_json::from_str(&v.order_json).ok()?;
            (order.get("sitting_id").and_then(Value::as_str) == Some(sitting_id)).then_some(Round { view: v, order })
        })
        .collect();
    out.sort_by_key(|r| r.int("created_at_ms"));
    out
}

/// Σ the totals of the rounds that took money.
pub fn bill(rounds: &[Round<'_>]) -> i64 {
    rounds.iter().filter(|r| r.billed()).map(|r| r.int("total")).sum()
}

/// Σ what has been paid against those rounds.
pub fn paid(rounds: &[Round<'_>]) -> i64 {
    rounds.iter().filter(|r| r.billed()).map(|r| r.int("paid")).sum()
}

/// Is this sitting still at its table? A round still cooking, or a bill not
/// yet paid in full, keeps it open.
pub fn open(rounds: &[Round<'_>]) -> bool {
    rounds.iter().any(|r| !is_terminal(r.status())) || paid(rounds) < bill(rounds)
}

/// One sitting as the waiter's screen draws it.
pub fn card(sitting_id: &str, rounds: &[Round<'_>]) -> Value {
    let table = rounds
        .iter()
        .rev()
        .find_map(|r| r.order.pointer("/fulfilment/table").and_then(Value::as_str))
        .unwrap_or("");
    let (b, p) = (bill(rounds), paid(rounds));
    json!({
        "sitting_id": sitting_id,
        "table": table,
        "bill": b, "paid": p, "due": b - p,
        "rounds": rounds.iter().map(|r| json!({
            "id": r.view.order_id, "seq": r.view.seq, "status": r.status(),
            "items": r.order.get("items"), "subtotal": r.int("subtotal"),
            "discount": r.int("discount"), "tip": r.int("tip"), "total": r.int("total"),
            "paid": r.int("paid"), "payment_status": r.order.get("payment_status"),
            "placed_by": r.order.get("placed_by"), "amended": r.order.get("amended"),
            "adjustments": r.order.get("adjustments"), "created_at_ms": r.int("created_at_ms"),
        })).collect::<Vec<_>>(),
    })
}

/// THE ROOM: every OPEN sitting, grouped from one pass over the projection the
/// console already reads (`orders_view`, memoised per generation).
pub fn room(listed: &[OrderView]) -> Vec<Value> {
    let mut ids: Vec<String> = Vec::new();
    for v in listed {
        let Ok(o) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        if let Some(s) = o.get("sitting_id").and_then(Value::as_str) {
            if !ids.iter().any(|x| x == s) {
                ids.push(s.to_string());
            }
        }
    }
    ids.iter()
        .filter_map(|s| {
            let rs = rounds(listed, s);
            open(&rs).then(|| card(s, &rs))
        })
        .collect()
}

#[cfg(test)]
mod tests;
