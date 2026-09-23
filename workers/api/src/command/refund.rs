//! END AN ACCEPTED ORDER: the refund, in one object turn (roadmap A10,
//! BLUEPRINT-OPERATIONAL-BLIND-SPOTS-2026-09-23 §2.4, P1-4).
//!
//! THE MISSING EXIT (memory `dowiz-order-fsm-has-no-exit`). `CANCELLED` and
//! `REJECTED` are reachable only from `PENDING`; past it the kernel's one exit
//! that is not "forward to DELIVERED" is `REFUNDING`, from CONFIRMED,
//! PREPARING, READY and IN_DELIVERY, and from there only `COMPENSATED_REFUND`.
//! Nothing emitted it, so an abandoned order sat in the queue for ever.
//!
//! WHAT IS OWED BACK IS WHAT WAS TAKEN, read off the order's own evidence:
//! the room's `payments[]` (in the order's currency, `pay::settles`), the card
//! webhook's `amount_received`, the courier's `cash_collected`. `took_money`
//! is NOT that question — it is whether an order counts in the takings, and it
//! says yes for a cash order nobody has paid yet.
//!
//! NO MONEY TAKEN IS A CANCELLATION, and the kernel offers exactly one path
//! for it past PENDING: `→ REFUNDING → COMPENSATED_REFUND`, both edges in this
//! turn, so the order leaves the takings at once (`took_money` excludes
//! COMPENSATED_REFUND). With money taken the order stays REFUNDING — money in
//! flight still counts — until a person says it was handed back
//! (`complete: true`), which is the second edge.
//!
//! THE SHELF. Food the kitchen never took is released; food it took was
//! `Consumed` at PREPARING and is left alone — whether it comes back as
//! `Wasted{Returned}` or resellable stock is the owner's call, never inferred.
//!
//! LAW 3: nothing here writes `items`, `subtotal`, `discount`, `tip` or
//! `total`. The merged order is the old one with a status, a timestamp and a
//! `refund` record added.

use super::amend::next_seq;
use super::pay::settles;
use super::room_rules::OTHER_MAX_CHARS;
use super::Refused;
use crate::hubdo::OrderView;
use dowiz_hub::EventKind;
use dowiz_kernel::order_machine::{assert_transition, OrderStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Why an order was refunded: a word the owner can count, as `VoidReason`.
/// `refused_at_door` is the one the blueprint names (P1-4); the rest are the
/// memo's abandoned orders (customer rang off, venue closed, no courier).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefundReason {
    RefusedAtDoor,
    VenueCancelled,
    CustomerRequest,
    PaymentError,
    Other(String),
}

impl RefundReason {
    pub fn parse(s: &str) -> Option<RefundReason> {
        match s.trim() {
            "refused_at_door" => Some(RefundReason::RefusedAtDoor),
            "venue_cancelled" => Some(RefundReason::VenueCancelled),
            "customer_request" => Some(RefundReason::CustomerRequest),
            "payment_error" => Some(RefundReason::PaymentError),
            other => {
                let text = other.strip_prefix("other:")?.trim();
                (!text.is_empty() && text.chars().count() <= OTHER_MAX_CHARS)
                    .then(|| RefundReason::Other(text.to_string()))
            }
        }
    }

    pub fn word(&self) -> String {
        match self {
            RefundReason::RefusedAtDoor => "refused_at_door".into(),
            RefundReason::VenueCancelled => "venue_cancelled".into(),
            RefundReason::CustomerRequest => "customer_request".into(),
            RefundReason::PaymentError => "payment_error".into(),
            RefundReason::Other(t) => format!("other:{t}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundIn {
    pub order_id: String,
    /// The venue that was AUTHORISED; checked against the order's own.
    pub location_id: String,
    /// The signer. Mandatory, and never waived.
    pub by: String,
    /// A `RefundReason` word. Required to START a refund; not read to complete one.
    #[serde(default)]
    pub reason: String,
    /// `false`: start the refund. `true`: the money was handed back — the
    /// order goes REFUNDING → COMPENSATED_REFUND.
    #[serde(default)]
    pub complete: bool,
    pub now_ms: i64,
    /// THE COURIER'S TAP (§2.4 `refused_at_door`): set only by the courier
    /// route, which has already checked the assignment is the signer's. The
    /// order must be on the road, the reason is the door, and the courier's
    /// `cash_collected: 0` goes on the order in the same turn.
    #[serde(default)]
    pub at_door: bool,
    /// What the courier (or the owner) saw, in their words: "nobody opened",
    /// "said they never ordered". Optional, never required; bounded.
    #[serde(default)]
    pub note: Option<String>,
}

/// A note is a sentence, not a document.
pub const NOTE_MAX_CHARS: usize = 280;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundOut {
    /// The order as it now is.
    pub merged: String,
    /// The seq of the LAST event written.
    pub seq: u64,
    pub generation: i64,
}

/// One event to append: kind, delta body, seq.
pub type Written = (EventKind, String, u64);

/// What the venue physically took for this order, in the order's minor units,
/// and whether it took anything. Σ of every source; each is written by a
/// different path and none writes another's field.
pub fn money_taken(order: &Value) -> Result<(i64, bool), Refused> {
    let over = || Refused::Invalid("the order's payments overflow".into());
    let mut owed: i64 = 0;
    let mut any = false;
    for p in order.get("payments").and_then(Value::as_array).into_iter().flatten() {
        owed = owed.checked_add(settles(p)).ok_or_else(over)?;
        any = true;
    }
    // The card webhook records what it RECEIVED (stripe.rs); a card order
    // marked paid with no such record is owed its total.
    if let Some(card) = order.get("amount_received").and_then(Value::as_i64) {
        owed = owed.checked_add(card).ok_or_else(over)?;
        any = true;
    } else if !any && order.get("payment_status").and_then(Value::as_str) == Some("paid") {
        owed = owed.checked_add(order.get("total").and_then(Value::as_i64).unwrap_or(0)).ok_or_else(over)?;
        any = true;
    }
    if let Some(cash) = order.get("cash_collected").and_then(Value::as_i64).filter(|c| *c > 0) {
        owed = owed.checked_add(cash).ok_or_else(over)?;
        any = true;
    }
    Ok((owed, any && owed > 0))
}

/// Did the kitchen take this order? Then its ingredients were `Consumed` at
/// PREPARING and must not be released. An order that went CONFIRMED →
/// IN_DELIVERY (a bottle, no cooking) never took that edge: `at.PREPARING`.
pub fn kitchen_took(order: &Value) -> bool {
    let status = order.get("status").and_then(Value::as_str).unwrap_or("");
    matches!(status, "PREPARING" | "READY")
        || order.get("at").and_then(|a| a.get("PREPARING")).is_some()
}

/// The kernel decides the edge; `order` becomes `to`, stamped, and the delta
/// is the next event. Nothing but `status` and `at` moves.
fn step(order: &mut Value, to: OrderStatus, now_ms: i64) -> Result<String, Refused> {
    let from = order.get("status").and_then(Value::as_str).unwrap_or("");
    let from = OrderStatus::from_str(from)
        .ok_or_else(|| Refused::Conflict(format!("unknown status {from:?}")))?;
    assert_transition(from, to).map_err(|e| Refused::Conflict(e.message()))?;
    let before = order.clone();
    order["status"] = json!(to.as_str());
    crate::live_eta::stamp(order, to.as_str(), now_ms);
    Ok(crate::fold::delta(&before, order).to_string())
}

/// The refund's note, added to the record `decide` just wrote.
fn note_of(order: &mut Value, text: &str) -> String {
    note(order, &["refund", "note"], json!(text))
}

/// A fact added without the status moving: `Noted`.
fn note(order: &mut Value, path: &[&str], value: Value) -> String {
    let before = order.clone();
    let mut at = &mut *order;
    for k in &path[..path.len() - 1] {
        if !at.get(*k).is_some_and(Value::is_object) {
            at[*k] = json!({});
        }
        at = &mut at[*k];
    }
    at[path[path.len() - 1]] = value;
    crate::fold::delta(&before, order).to_string()
}

/// THE WHOLE REFUND, over two images already in memory. Both are mutated in
/// place and written by the caller only if this returns `Ok`; every refusal
/// comes before the first append.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    stock: &mut dowiz_hub::stock::StockLog,
    current: Option<&OrderView>,
    input: &RefundIn,
) -> Result<(Value, Vec<Written>), Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    let old: Value = serde_json::from_str(&current.order_json)
        .map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;
    if old.get("location_id").and_then(Value::as_str) != Some(input.location_id.as_str()) {
        return Err(Refused::NotFound);
    }
    if input.by.trim().is_empty() {
        return Err(Refused::Invalid("a refund names who is refunding".into()));
    }
    let status = old.get("status").and_then(Value::as_str).unwrap_or("").to_string();

    // Every edge and record is computed in memory first; `events` is appended
    // only once all of them passed.
    let mut order = old.clone();
    let mut events: Vec<(EventKind, String)> = Vec::new();
    let mut release = false;
    if input.complete && input.at_door {
        return Err(Refused::Invalid("the money handed back is recorded by the venue, not at the door".into()));
    }
    if input.complete {
        events.push((EventKind::Noted, note(&mut order, &["refund", "returned"], json!({"by": input.by, "at": input.now_ms}))));
        events.push((EventKind::Advanced, step(&mut order, OrderStatus::CompensatedRefund, input.now_ms)?));
    } else {
        let said = input.note.as_deref().map(str::trim).filter(|n| !n.is_empty());
        if said.is_some_and(|n| n.chars().count() > NOTE_MAX_CHARS) {
            return Err(Refused::Invalid(format!("a note is at most {NOTE_MAX_CHARS} characters")));
        }
        let reason = RefundReason::parse(&input.reason)
            .ok_or_else(|| Refused::Invalid(format!("{:?} is not a refund reason", input.reason)))?;
        if status == "PENDING" {
            return Err(Refused::Conflict("a PENDING order is rejected or cancelled, not refunded".into()));
        }
        if input.at_door && reason != RefundReason::RefusedAtDoor {
            return Err(Refused::Invalid("a courier's refund is a refusal at the door".into()));
        }
        if input.at_door && status != "IN_DELIVERY" {
            return Err(Refused::Conflict(format!("this order is {status}, not at the door")));
        }
        events.push((EventKind::Advanced, step(&mut order, OrderStatus::Refunding, input.now_ms)?));
        let (owed, taken) = money_taken(&old)?;
        events.push((EventKind::Noted, note(&mut order, &["refund"], json!({
            "reason": reason.word(), "by": input.by, "at": input.now_ms, "from": status,
            "owed": owed, "money_taken": taken, "currency": old.get("currency"),
        }))));
        if let Some(n) = said {
            events.push((EventKind::Noted, note_of(&mut order, n)));
        }
        // Nothing was taken at the door, said by the courier who was there:
        // law 4 then reads the run as zero cash owed, not as never ended.
        if input.at_door {
            events.push((EventKind::Noted, note(&mut order, &["cash_collected"], json!(0))));
        }
        // Nothing to hand back: the refund is complete in this same turn.
        if !taken {
            events.push((EventKind::Advanced, step(&mut order, OrderStatus::CompensatedRefund, input.now_ms)?));
        }
        release = !kitchen_took(&old);
    }

    let mut written = Vec::with_capacity(events.len());
    let mut seq = current.seq;
    for (kind, body) in events {
        seq = next_seq(seq, input.now_ms);
        hub.append(kind, &input.order_id, &body, seq, [0u8; 32])
            .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
        written.push((kind, body, seq));
    }
    if release {
        let led = stock.ledger().map_err(|e| Refused::Stock(e.to_string()))?;
        let evs = dowiz_hub::stock::settle(&led, &input.order_id, false);
        if !evs.is_empty() {
            stock.append_all(&evs).map_err(|e| Refused::Stock(e.to_string()))?;
        }
    }
    Ok((order, written))
}

/// The owner's choice for the food a refused delivery brought back.
pub mod returned;

#[cfg(test)]
mod tests;
