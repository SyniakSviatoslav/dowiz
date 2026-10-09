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
    /// round took no money, and a guest is not asked to pay for it. Nor for a
    /// guest's QR round nobody has confirmed (D9): the code on the table never
    /// expires, and `pay` refuses that round until a waiter confirms it.
    pub fn billed(&self) -> bool {
        let waiting = self.status() == "PENDING"
            && self.order.get("placed_by").and_then(Value::as_str) == Some(dowiz_hub::room::pay::GUEST);
        took_money(self.status()) && !waiting
    }
}

/// EVERY SITTING AT ONCE, each with its rounds oldest first, in the order its
/// first round appears in `listed` (W-LOOPA row 1). ONE parse per order: the
/// floor, the room and the QR placement used to collect the sitting ids and
/// then call [`rounds`] per sitting, which parsed every order AGAIN -- S x N
/// parses, 32 ms at 100 open orders and 279 ms at 300 (R-LOOPS B3), three to
/// twenty-eight times the Worker's 10 ms cap on every staff floor poll. The
/// grouping is the same as [`rounds`]': same skip of an unparsable envelope or
/// a missing `sitting_id`, same stable sort by `created_at_ms`
/// (`sitting/tests/equiv.rs` holds the old code as the oracle).
pub fn sittings_of(listed: &[OrderView]) -> Vec<(String, Vec<Round<'_>>)> {
    sittings_where(listed, |_| true)
}

/// [`sittings_of`] over the orders `keep` admits, judged on the order already
/// parsed -- so a caller that filters (the QR placer, by venue) parses once too.
pub fn sittings_where<'a>(listed: &'a [OrderView], keep: impl Fn(&Value) -> bool) -> Vec<(String, Vec<Round<'a>>)> {
    let mut at: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut out: Vec<(String, Vec<Round<'a>>)> = Vec::new();
    for v in listed {
        let Ok(order) = serde_json::from_str::<Value>(&v.order_json) else { continue };
        if !keep(&order) {
            continue;
        }
        let i = {
            let Some(s) = order.get("sitting_id").and_then(Value::as_str) else { continue };
            match at.get(s) {
                Some(&i) => i,
                None => {
                    at.insert(s.to_string(), out.len());
                    out.push((s.to_string(), Vec::new()));
                    out.len() - 1
                }
            }
        };
        out[i].1.push(Round { view: v, order });
    }
    for (_, rs) in &mut out {
        rs.sort_by_key(|r| r.int("created_at_ms"));
    }
    out
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

/// What has been paid against ONE round: Σ its recorded `payments`, each in
/// the order's currency (`pay::settles`). NOTHING WRITES AN ORDER-LEVEL
/// `paid` — `command::pay` only appends to `payments[]` — and reading one made
/// every live sitting read 0 paid: a paid-up table never left the room, and
/// the reprice guard "paid > total" never fired.
pub fn paid_of(order: &Value) -> i64 {
    order.get("payments").and_then(Value::as_array).into_iter().flatten().map(super::pay::settles).sum()
}

/// Σ what has been paid against those rounds.
pub fn paid(rounds: &[Round<'_>]) -> i64 {
    rounds.iter().filter(|r| r.billed()).map(|r| paid_of(&r.order)).sum()
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
            "paid": paid_of(&r.order), "payments": r.order.get("payments"),
            "payment_status": r.order.get("payment_status"),
            "placed_by": r.order.get("placed_by"), "amended": r.order.get("amended"),
            "adjustments": r.order.get("adjustments"), "created_at_ms": r.int("created_at_ms"),
        })).collect::<Vec<_>>(),
    })
}

/// THE ROOM: every OPEN sitting, grouped from one pass over the projection the
/// console already reads (`orders_view`, memoised per generation).
pub fn room(listed: &[OrderView]) -> Vec<Value> {
    sittings_of(listed).into_iter().filter_map(|(s, rs)| open(&rs).then(|| card(&s, &rs))).collect()
}

#[cfg(test)]
mod tests;
#[cfg(test)]
#[path = "sitting/tests/equiv.rs"]
mod equiv;
