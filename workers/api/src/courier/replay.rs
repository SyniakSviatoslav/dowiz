//! THE COURIER'S TAP THAT ALREADY RAN (W-O2, 2026-09-28; the rule is
//! `idempotency::commit`).
//!
//! THE DEFECT (W-AUDIT O2, courier half). A courier's tap is claimed by the
//! Worker, written by the object, and answered by the Worker afterwards. A
//! reply lost after the object wrote -- the isolate cut off, the stub fetch
//! failed on the way back -- released the claim (a 5xx is not an answer), and
//! the tap replayed out of the phone's outbox ran AGAIN: a second `Noted`
//! event re-stamping `accepted_at_ms`, a pickup stamp moved to the retry's
//! clock, a delivery answered with the FSM's "illegal edge" and its shift
//! write never made -- and that 409 recorded as the key's answer, shortfall
//! lost.
//!
//! THE FIX, per route, the placement's (`storefront::place`):
//!   * the decisive object write carries the claim (`commit::Claimed` into
//!     `/fold/advance` and `/fold/refund`, `commit::Marked` into
//!     `/fold/append`), and the object marks it committed in the turn that
//!     wrote the log;
//!   * a retry told `Committed` answers from that output FIRST, before any
//!     check or write;
//!   * the ops writes that follow the log write are ONCE-ONLY, so a retry that
//!     finishes the first call's tail writes nothing the first call wrote.
//!
//! PURE: answers and table operations; the handlers are `courier.rs` and
//! `courier/door.rs`.

use dowiz_hub::table::Table;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use super::{cash, field_i64, K_ASG, K_SHIFT};
use crate::command::refund::RefundOut;

/// A committed output, read as what its route stored. `None` when there is
/// none, or it does not read -- then the route runs, as it did before.
pub(crate) fn committed<T: DeserializeOwned>(output: Option<&str>) -> Option<T> {
    output.and_then(|o| serde_json::from_str(o).ok())
}

/// `accept`'s answer, on the first call and on a committed retry alike.
pub(crate) fn accept_answer(order_id: &str, cash_due: i64) -> Value {
    json!({ "ok": true, "orderId": order_id, "cashDue": cash_due })
}

/// `deliver`'s answer: the order, and the cash with its shortfall.
pub(crate) fn deliver_answer(order: &Value, cash_due: i64, handed: &cash::Handover) -> Value {
    json!({ "order": order, "cashDue": cash_due, "cashCollected": handed.collected, "short": handed.short })
}

/// `refused`'s answer, from the refund the object wrote.
pub(crate) fn refused_answer(out: &RefundOut) -> Value {
    json!({ "order": serde_json::from_str::<Value>(&out.merged).unwrap_or(Value::Null), "seq": out.seq })
}

/// A stamp that is there: present and not null.
fn stamped(a: &Value, k: &str) -> bool {
    a.get(k).is_some_and(|v| !v.is_null())
}

/// The pickup stamp on the assignment. `Ok(true)` when written.
pub(crate) fn stamp_pickup(t: &mut Table, order_id: &str, now: i64) -> Result<bool, String> {
    let Some(mut a) = t.get(K_ASG, order_id).and_then(|j| serde_json::from_str::<Value>(&j).ok()) else {
        return Ok(false);
    };
    // ONCE: a retry finishing the first call's tail keeps the first stamp.
    if stamped(&a, "picked_up_at_ms") {
        return Ok(false);
    }
    a["picked_up_at_ms"] = json!(now);
    t.put(K_ASG, order_id, &a.to_string(), &[], &[]).map_err(|e| format!("assignment: {e}"))?;
    Ok(true)
}

/// The delivery on the assignment and the courier's open shift, in one write.
/// `Ok(true)` when written.
pub(crate) fn settle_delivery(
    t: &mut Table,
    order_id: &str,
    courier_id: &str,
    now: i64,
    collected: i64,
) -> Result<bool, String> {
    let asg = t.get(K_ASG, order_id).and_then(|j| serde_json::from_str::<Value>(&j).ok());
    // ONCE: a delivery already on the assignment was counted on the shift in
    // the same write, so a retry finishing the tail writes neither.
    if asg.as_ref().is_some_and(|a| stamped(a, "delivered_at_ms")) {
        return Ok(false);
    }
    if let Some(mut a) = asg {
        a["delivered_at_ms"] = json!(now);
        a["cash_collected"] = json!(collected);
        t.put(K_ASG, order_id, &a.to_string(), &[], &[]).map_err(|e| format!("assignment: {e}"))?;
    }
    if let Some(mut s) = t
        .get(K_SHIFT, courier_id)
        .and_then(|j| serde_json::from_str::<Value>(&j).ok())
        .filter(|s| s.get("ended_at_ms").map_or(true, Value::is_null))
    {
        let (Some(n), Some(cash)) =
            (cash::add(field_i64(&s, "deliveries"), 1), cash::add(field_i64(&s, "cash_collected"), collected))
        else {
            return Err("shift: its totals would overflow".into());
        };
        s["deliveries"] = json!(n);
        s["cash_collected"] = json!(cash);
        t.put(K_SHIFT, courier_id, &s.to_string(), &[], &[]).map_err(|e| format!("shift: {e}"))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
