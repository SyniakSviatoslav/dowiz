//! "SEEN BY THE KITCHEN": the ack, and the fold that lists what nobody saw
//! (BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23 §2.6, P1-6; LAST-MILE §3.1).
//!
//! THE DEFECT. The tablet says "sent to the kitchen" while the printer is out
//! of paper or the wifi is down. "Sent" was one word for a thing nobody had
//! recorded, so nobody knew the ticket never arrived.
//!
//! WHAT REPLACES IT. The kitchen taps the ticket and the console POSTs
//! `/api/staff/orders/:id/kitchen-ack`; the venue's object writes ONE `Noted`
//! event, `{kitchen: {seen: {by, at}}}`, onto the order. `unconfirmed` lists the
//! live orders placed more than `UNSEEN_AFTER_MS` ago that carry no such fact,
//! and `/api/owner/health` shows them.
//!
//! ONE SOURCE OF TRUTH, AND IT IS THE ORDER LOG, NOT THE OUTBOX. The outbox is
//! the wrong place for "seen", for three measured reasons (`outbox/rails.rs`):
//! a delivered entry is REMOVED (`Verdict::Sent` → `t.remove`), so a fold over
//! the outbox cannot list a ticket that was delivered and never seen — exactly
//! the case this exists for; and an abandoned entry is removed too (the
//! `print` kind, when it came, is pulled by the printer rather than drained,
//! and its entry is removed on the DELETE just the same). The order log
//! keeps every order for its whole hot life, and the ack is a fact ABOUT the
//! order, so it is written where the order is. The print rail
//! (`print_rail.rs`) does the same: the printer's `DELETE` becomes
//! `kitchen.printed` on the ORDER, beside `kitchen.seen`, because its outbox
//! entry is removed the moment it prints. Printed does not count as seen.
//!
//! A REPEATED ACK IS ONE FACT. The first tap is what "seen 18:03" means; a
//! second tap (a retry without an idempotency key, or a second cook) writes
//! NOTHING and answers the order as it stands. Two notes would make "seen"
//! mean the last tap, which is not when the kitchen first knew.
//!
//! PURE. `decide` and `unconfirmed` are functions of their arguments; the
//! object (`hubdo/kitchen_ack.rs`) writes, and the route authenticates.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::amend::next_seq;
use super::Refused;
use crate::hubdo::OrderView;

/// How long a placed order may go without a "seen" before the owner is told.
///
/// THREE MINUTES, AND THE NUMBER IS THE DELIVERY FLOOR'S, NOT A GUESS. The
/// Haiku draft had 30 s, which is shorter than the rail it measures: the bell
/// is delivered by the MINUTE cron (LAST-MILE §3.1 "today's 0-60 s cron
/// floor"), and a first failed attempt waits `outbox::backoff_ms(1)` = 10 s and
/// then the next tick — so a healthy but unlucky ticket reaches the kitchen
/// within two ticks, 120 s. 30 s would have listed most tickets on a quiet
/// afternoon, and an alarm that is always on is one nobody reads. Two ticks
/// plus one minute for a human hand is 180 s; §2.6 asks for "N minutes".
/// When the polling printer (§3.1 step 1, a 5 s poll) replaces the cron, this
/// can come down.
pub const UNSEEN_AFTER_MS: i64 = 180_000;

/// The statuses at which a ticket is owed a "seen". Past them a kitchen has
/// already ACTED on the order (it is cooking, ready, out, done), which is
/// stronger evidence than a tap; terminal and SCHEDULED orders owe nothing now.
const OWED: &[&str] = &["PENDING", "CONFIRMED"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenAckIn {
    pub order_id: String,
    pub location_id: String,
    pub by: String,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KitchenAckOut {
    /// The order as it now is.
    pub merged: String,
    /// The order's version: the event just written, or the one it already had.
    pub seq: u64,
    pub generation: i64,
    /// False when the order had already been seen and nothing was written.
    pub fresh: bool,
}

/// What `decide` wants written: the delta body and its seq. `None` = nothing.
pub type Write = Option<(String, u64)>;

/// Whether this order already carries a "seen".
pub fn seen(order: &Value) -> bool {
    order.pointer("/kitchen/seen/at").is_some()
}

/// Record that the kitchen saw the ticket. Appends at most one `Noted`.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    current: Option<&OrderView>,
    input: &KitchenAckIn,
) -> Result<(Value, Write), Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    let old: Value = serde_json::from_str(&current.order_json)
        .map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;
    // ANOTHER VENUE'S ORDER IS NOT FOUND, never "forbidden": the answer must
    // not say that it exists somewhere.
    if old.get("location_id").and_then(Value::as_str) != Some(input.location_id.as_str()) {
        return Err(Refused::NotFound);
    }
    if input.by.trim().is_empty() {
        return Err(Refused::Invalid("a kitchen ack must name who saw it".into()));
    }
    if seen(&old) {
        return Ok((old, None));
    }
    let mut order = old.clone();
    let mut kitchen = order.get("kitchen").cloned().filter(Value::is_object).unwrap_or_else(|| json!({}));
    kitchen["seen"] = json!({ "by": input.by, "at": input.now_ms });
    order["kitchen"] = kitchen;
    let body = crate::fold::delta(&old, &order).to_string();
    let seq = next_seq(current.seq, input.now_ms);
    hub.append(dowiz_hub::EventKind::Noted, &input.order_id, &body, seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok((order, Some((body, seq))))
}

/// One ticket nobody has said they saw.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Unseen {
    pub order_id: String,
    pub status: String,
    /// Milliseconds since the order was placed.
    pub age_ms: i64,
    /// When the kitchen printer said it printed the ticket, if it did
    /// (`print_rail.rs`). PRINTED IS NOT SEEN (§2.6: three records): a
    /// printed, unseen ticket stays on this list, and this field is the
    /// difference between "check the printer" and "check the pass".
    pub printed_at: Option<i64>,
}

/// The owed orders placed at least `UNSEEN_AFTER_MS` ago with no "seen",
/// oldest first. `orders` are FOLDED orders (`hubstore::orders_state`).
pub fn unconfirmed<'a>(orders: impl IntoIterator<Item = &'a Value>, now_ms: i64) -> Vec<Unseen> {
    let mut out: Vec<Unseen> = orders
        .into_iter()
        .filter_map(|o| {
            let status = o.get("status").and_then(Value::as_str)?;
            // An order with no placement instant cannot be aged; it is not
            // guessed at (0 would make it 56 years late).
            let placed = o.get("created_at_ms").and_then(Value::as_i64)?;
            let age_ms = now_ms - placed;
            (OWED.contains(&status) && !seen(o) && age_ms >= UNSEEN_AFTER_MS).then(|| Unseen {
                order_id: o.get("id").and_then(Value::as_str).unwrap_or_default().to_string(),
                status: status.to_string(),
                age_ms,
                printed_at: o.pointer("/kitchen/printed/at").and_then(Value::as_i64),
            })
        })
        .collect();
    out.sort_by(|a, b| b.age_ms.cmp(&a.age_ms).then_with(|| a.order_id.cmp(&b.order_id)));
    out
}

/// `unconfirmed` over a venue's log, as `/api/owner/health` asks it.
pub fn unconfirmed_in(hub: &dowiz_hub::Hub, now_ms: i64) -> Vec<Unseen> {
    let orders: Vec<Value> = crate::hubstore::orders_state(hub)
        .into_iter()
        .filter_map(|e| {
            let mut o = serde_json::from_str::<Value>(&e.order_json).ok()?;
            // The log's subject is the id; the body may not repeat it.
            if o.get("id").is_none() && o.is_object() {
                o["id"] = json!(e.order_id);
            }
            Some(o)
        })
        .collect();
    unconfirmed(orders.iter(), now_ms)
}

#[cfg(test)]
mod tests;
