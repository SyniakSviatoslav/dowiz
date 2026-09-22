//! ADVANCE AN ORDER, AND SETTLE THE SHELF, IN ONE TURN.
//!
//! THE DEFECT. `owner::order_action` appended the transition through
//! `append_for` and then, in a SECOND round trip, settled the stock ledger:
//! PREPARING consumes what was held, REJECTED and CANCELLED release it. The
//! settlement's failure was logged and deliberately did not fail the
//! transition, with this reasoning at the call site — "the order has already
//! moved and the customer has been told; refusing now would leave the order and
//! the ledger disagreeing in the other direction."
//!
//! THAT REASONING WAS CORRECT AND THE SITUATION IT REASONED ABOUT SHOULD NOT
//! EXIST. Both outcomes are an order and a ledger that disagree; the choice was
//! only which way. Here the two are one turn: the log and the ledger are
//! changed against copies in memory and written after both have succeeded, so
//! neither direction of disagreement is reachable by a failure in the other.
//!
//! THE KERNEL STILL DECIDES, and it decides HERE now rather than in the Worker.
//! `json_api::apply_event_logic` is linked into this crate, so the object can
//! call it; an illegal edge is the kernel's refusal, exactly as before, and it
//! reaches the customer as a 409 carrying the kernel's own words.

use super::Refused;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// What the object needs to advance one order.
///
/// `next` IS A STATUS AND THE ACTION IS NOT SENT. `order_action` maps an intent
/// ("confirm", "collected") to a status in ONE place and the FSM decides
/// whether the edge is legal; sending the intent would put that map on both
/// sides of the boundary, which is how a vocabulary ends up hand-copied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceIn {
    pub order_id: String,
    /// The venue that was AUTHORISED, not the one the token happens to name.
    /// Checked against the order's own `location_id` below: an owner of two
    /// venues must not move the other one's order.
    pub location_id: String,
    pub next: String,
    pub reason: Option<String>,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvanceOut {
    /// The order as it now is — the merged record, not the delta. It is what
    /// the console draws.
    pub merged: String,
    pub generation: i64,
}

/// Does this transition consume the ingredients, release them, or neither?
///
/// PREPARING consumes: the food is being made and those ingredients are gone.
/// REJECTED and CANCELLED release: nothing was cooked, and holding them would
/// strand the difference for ever. Everything else leaves the shelf alone —
/// including PICKED_UP and DELIVERED, because by then the consumption already
/// happened at PREPARING and settling twice would double-count it.
pub fn settlement(next: &str) -> Option<bool> {
    match next {
        "PREPARING" => Some(true),
        "REJECTED" | "CANCELLED" => Some(false),
        _ => None,
    }
}

/// THE WHOLE TRANSITION, over two images already in memory.
///
/// `current` is the order as the object's own projection has it. `None` and
/// "belongs to another venue" are THE SAME ANSWER on purpose: a 404 that can be
/// told apart from a 403 tells an owner of one venue which order ids exist at
/// another.
///
/// THE SHELF IS SETTLED AFTER THE ORDER MOVED, never before. The kernel owns
/// whether the transition is legal at all, and taking ingredients off the shelf
/// for an edge it then refuses is a loss with no order behind it. Both are in
/// memory, so "after" costs nothing and still means nothing is written until
/// both have succeeded.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    stock: &mut dowiz_hub::stock::StockLog,
    current: Option<&str>,
    input: &AdvanceIn,
) -> Result<Value, Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    let old: Value = serde_json::from_str(current).unwrap_or(json!({}));
    if old.get("location_id").and_then(Value::as_str) != Some(input.location_id.as_str()) {
        return Err(Refused::NotFound);
    }

    // The kernel decides. An illegal edge is its refusal, not ours.
    let updated = dowiz_kernel::json_api::apply_event_logic(current, &input.next)
        .map_err(Refused::Conflict)?;
    let mut merged: Value = serde_json::from_str(&updated)
        .map_err(|e| Refused::Append(format!("kernel order json unreadable: {e}")))?;
    crate::hubstore::carry_over(&old, &mut merged);
    crate::live_eta::stamp(&mut merged, &input.next, input.now_ms);
    // A rejection carries WHY, recorded with the event so the customer can be
    // told something true rather than "rejected".
    if input.next == "REJECTED" {
        merged["rejection_reason"] = json!(input.reason);
    }

    // The delta against the state this transition started from.
    let body = crate::fold::delta(&old, &merged).to_string();
    hub.append(
        dowiz_hub::EventKind::Advanced,
        &input.order_id,
        &body,
        input.now_ms as u64,
        [0u8; 32],
    )
    .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;

    if let Some(consume) = settlement(&input.next) {
        let led = stock.ledger().map_err(|e| Refused::Stock(e.to_string()))?;
        let evs = dowiz_hub::stock::settle(&led, &input.order_id, consume);
        if !evs.is_empty() {
            stock.append_all(&evs).map_err(|e| Refused::Stock(e.to_string()))?;
        }
    }
    Ok(merged)
}
