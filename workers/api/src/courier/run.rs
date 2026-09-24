//! THE RUN IN HAND: which of a courier's assignments they are still carrying.
//!
//! THE DEFECT (A10's hand-over). An assignment is "open" while its
//! `delivered_at_ms` is null, and only `deliver` writes that field. A refund of
//! an IN_DELIVERY order moves the ORDER to REFUNDING / COMPENSATED_REFUND and
//! leaves the ops image alone -- the refund is one turn over the log and the
//! shelf, and a third image in it would break the one-image rule -- so the
//! assignment stayed open for ever and `shift(open: false)` refused the
//! courier with "finish the delivery in hand" for a delivery nobody could
//! finish: the kernel has no edge from REFUNDING to DELIVERED.
//!
//! THE FIX IS IN THE READER, not a second write. A run is in hand while its
//! ORDER can still reach the door; the order's status lives in the log, the
//! assignment in the ops image, and the shift handler reads both before it
//! writes the one it owns.
//!
//! AN UNREADABLE OR MISSING ORDER IS STILL HELD. Wrongly releasing a run
//! strands food on the road with nobody holding it; wrongly holding one is a
//! courier who cannot clock off and says so -- visible, and fixable.

use dowiz_kernel::order_machine::OrderStatus;
use serde_json::Value;

/// Is this order past the point where a courier can carry it anywhere?
///
/// The kernel's terminal set (DELIVERED, PICKED_UP, REJECTED, CANCELLED,
/// COMPENSATED_REFUND) plus REFUNDING: not terminal -- the money may still be
/// in flight -- but its only edge is COMPENSATED_REFUND, so the food is no
/// longer a delivery.
pub(crate) fn run_over(status: &str) -> bool {
    OrderStatus::from_str(status)
        .is_some_and(|s| s.is_terminal() || s == OrderStatus::Refunding)
}

/// The courier's assignments with no delivery recorded, by order id. These are
/// the candidates; `in_hand` removes the ones whose order has ended.
pub(crate) fn open_runs(rows: &[(String, String)], courier_id: &str) -> Vec<String> {
    rows.iter()
        .filter_map(|(id, j)| serde_json::from_str::<Value>(j).ok().map(|v| (id, v)))
        .filter(|(_, v)| {
            v.get("courier_id").and_then(Value::as_str) == Some(courier_id)
                && v.get("delivered_at_ms").map_or(true, Value::is_null)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// The runs the courier is still carrying: open assignments whose order is
/// not in `over` (the ids the caller read as `run_over` from the log).
pub(crate) fn in_hand(rows: &[(String, String)], courier_id: &str, over: &[String]) -> Vec<String> {
    open_runs(rows, courier_id).into_iter().filter(|id| !over.contains(id)).collect()
}

/// May this courier tap "refused at the door" on this order? Only while the
/// assignment is theirs and no delivery is recorded on it.
pub(crate) fn may_refuse(rows: &[(String, String)], courier_id: &str, order_id: &str) -> bool {
    open_runs(rows, courier_id).iter().any(|id| id == order_id)
}

/// The ids among `read` whose order has ended, from `(order_id, order json)`
/// as the log answered; `None` (not found) and unreadable json stay held.
pub(crate) fn ended(read: &[(String, Option<String>)]) -> Vec<String> {
    read.iter()
        .filter(|(_, raw)| {
            raw.as_deref()
                .and_then(|r| serde_json::from_str::<Value>(r).ok())
                .and_then(|v| v.get("status").and_then(Value::as_str).map(run_over))
                .unwrap_or(false)
        })
        .map(|(id, _)| id.clone())
        .collect()
}

/// The assignment rows with every ended run taken out, parsed: what the
/// owner's console folds (`record::tally`). Only open runs are ever in
/// `over`, so a delivered row is never dropped here.
pub(crate) fn still_carried(rows: &[(String, String)], over: &[String]) -> Vec<Value> {
    rows.iter()
        .filter(|(id, _)| !over.contains(id))
        .filter_map(|(_, j)| serde_json::from_str::<Value>(j).ok())
        .collect()
}

/// What `claim` found.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Take {
    /// The order is this courier's after the call.
    Taken,
    /// Another courier holds it.
    Held,
    /// Nobody may take it in this status (the `accept` handler's 409).
    NotReady,
}

/// TAKE AN ORDER: write this courier's assignment row, or say why not.
///
/// THE DEFECT (2026-09-24). This refused whenever ANY row existed -- including
/// the one the owner's `assign_courier` wrote naming this very courier -- so a
/// courier could never accept an order handed to them: 409 "another courier
/// took this order", about themselves. A row naming THIS courier is theirs
/// already, and accepting it again changes nothing (the row, its cash and its
/// pickup stamp stay as written). A row naming anyone else still refuses.
///
/// STATUS. From the pool only READY and CONFIRMED may be taken (see the
/// handler). An order the owner already handed to THIS courier may also be
/// PREPARING -- `command::assign::HANDABLE` allows it -- and accepting it
/// pre-empts nobody and skips no kitchen step (PREPARING has no edge to
/// IN_DELIVERY; `pickup` waits for READY).
pub(crate) fn claim(
    t: &mut dowiz_hub::table::Table,
    order_id: &str,
    courier_id: &str,
    status: &str,
    now: i64,
    cash_due: i64,
) -> Result<Take, String> {
    if let Some(j) = t.get(super::K_ASG, order_id) {
        let holder = serde_json::from_str::<Value>(&j)
            .ok()
            .and_then(|v| v.get("courier_id").and_then(Value::as_str).map(str::to_owned));
        if holder.as_deref() != Some(courier_id) {
            return Ok(Take::Held);
        }
        return Ok(if matches!(status, "READY" | "CONFIRMED" | "PREPARING") {
            Take::Taken
        } else {
            Take::NotReady
        });
    }
    if !matches!(status, "READY" | "CONFIRMED") {
        return Ok(Take::NotReady);
    }
    let rec = serde_json::json!({
        "order_id": order_id, "courier_id": courier_id, "assigned_at_ms": now,
        "cash_due": cash_due, "picked_up_at_ms": Value::Null,
        "delivered_at_ms": Value::Null, "cash_collected": Value::Null,
    })
    .to_string();
    t.put(super::K_ASG, order_id, &rec, &[], &[]).map_err(|e| format!("assignment: {e}"))?;
    Ok(Take::Taken)
}

#[cfg(test)]
mod tests;
