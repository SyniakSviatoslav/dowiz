//! HAND AN ORDER TO A COURIER: the assignment and the event, in one turn.
//!
//! THE DEFECT, AND IT HAD NO COMPENSATION AT ALL. `owner::assign_courier` wrote
//! the assignment record into the `ops` table and then, in a second round trip,
//! appended the `Noted` event that puts `courier_id` on the order. If that
//! append failed, the assignment stood and the order never said it had a
//! courier: the console shows an unassigned order, the courier's own list shows
//! the job, and the next attempt to assign it answers "this order already has a
//! courier" — naming nobody the owner can see.
//!
//! THE FIRST WRITE IS ALSO THE LOCK, which is what made the split dangerous.
//! `t.get("asg", order_id).is_some()` is the check for a second courier, so the
//! assignment record has to land for the refusal to work at all, and it landed
//! BEFORE the thing that makes it visible. In one turn the lock and the event
//! are the same decision.
//!
//! WHO THE COURIER IS STAYS IN THE WORKER. The roster lives in the PLATFORM
//! object — a different object, which this one cannot read — so
//! "is this a courier of this venue, and are they active" is resolved before
//! the command is sent, exactly as pricing is for a placement. What the object
//! decides is what only it can see: whether this order is this venue's, whether
//! it is in a state that can be handed out, and whether somebody already has
//! it.

use super::Refused;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The statuses an order can be handed out from.
///
/// NOT DERIVED FROM `is_active`. `REFUNDING` is active and must not be given to
/// a courier, so this is a deliberately narrower set and is written out rather
/// than computed from a kernel predicate that means something else.
pub const HANDABLE: [&str; 3] = ["CONFIRMED", "PREPARING", "READY"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignIn {
    pub order_id: String,
    /// The venue that was AUTHORISED. See `advance::AdvanceIn`.
    pub location_id: String,
    /// Already resolved against the platform roster by the caller, and already
    /// known to be this venue's and active.
    pub courier_id: String,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignOut {
    pub generation: i64,
}

/// What the assignment record says, and what the order's delta says.
///
/// SPLIT OUT AND PURE so both halves can be read side by side: the two used to
/// be written by two different closures in two different round trips, and the
/// only thing that kept `cash_due` and `courier_id` consistent between them was
/// that one person wrote both on the same afternoon.
pub fn records(current: &Value, input: &AssignIn) -> (String, Value) {
    // CASH IS DUE ONLY IF THE ORDER IS CASH. A courier carrying a card order's
    // total as "cash due" would be asked to hand over money nobody gave them.
    let cash_due = if current.get("payment").and_then(Value::as_str) == Some("cash") {
        current.get("total").and_then(Value::as_i64).unwrap_or(0)
    } else {
        0
    };
    let record = json!({
        "order_id": input.order_id,
        "courier_id": input.courier_id,
        "assigned_at_ms": input.now_ms,
        "cash_due": cash_due,
        "picked_up_at_ms": Value::Null,
        "delivered_at_ms": Value::Null,
        "cash_collected": Value::Null,
    })
    .to_string();
    let mut updated = current.clone();
    updated["courier_id"] = json!(input.courier_id);
    updated["assigned_at_ms"] = json!(input.now_ms);
    (record, updated)
}

/// THE WHOLE ASSIGNMENT, over the log and the ops table already in memory.
///
/// `current` is the order from the object's projection; `None` and "another
/// venue's order" are the same 404, for the reason `advance::decide` gives.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    ops: &mut dowiz_hub::table::Table,
    current: Option<&str>,
    input: &AssignIn,
) -> Result<(), Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };
    let old: Value = serde_json::from_str(current).unwrap_or(json!({}));
    if old.get("location_id").and_then(Value::as_str) != Some(input.location_id.as_str()) {
        return Err(Refused::NotFound);
    }
    let status = old.get("status").and_then(Value::as_str).unwrap_or("");
    if !HANDABLE.contains(&status) {
        return Err(Refused::Conflict(
            "only an accepted order can be handed to a courier".into(),
        ));
    }
    // THE LOCK AND THE EVENT ARE NOW THE SAME DECISION. This read used to be a
    // separate round trip whose write had to land before the event could be
    // appended; here it is a look at a table this turn is holding.
    if ops.get("asg", &input.order_id).is_some() {
        return Err(Refused::Conflict("this order already has a courier".into()));
    }

    let (record, updated) = records(&old, input);
    ops.put("asg", &input.order_id, &record, &[], &[])
        .map_err(|e| Refused::Append(format!("assignment: {e}")))?;
    let body = crate::fold::delta(&old, &updated).to_string();
    hub.append(
        dowiz_hub::EventKind::Noted,
        &input.order_id,
        &body,
        input.now_ms as u64,
        [0u8; 32],
    )
    .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;
    Ok(())
}
