//! TAKE A PAYMENT: one `Paid` event with an amount, method, signer, and till.
//!
//! A check (sitting) can receive multiple payments until its total is covered.
//! Each payment is one `Paid` event on its order; the sitting's bill is the sum
//! of all order totals (minus voids), and the check closes when Σ payments ==
//! the bill. The payment split works by the room calling `pay` N times with N
//! amounts, and the object refusing if the sum would exceed the bill (G5).
//!
//! THE CONFIRMED FRAME (BLUEPRINT-POS-THE-ROOM §4.5 (2)). An order whose bill
//! is settled carries `payment_status: "paid"` and cannot be amended — taking
//! money off it is a refund (not this command).

use super::amend::next_seq;
use super::Refused;
use crate::hubdo::OrderView;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayIn {
    pub order_id: String,
    pub location_id: String,
    /// The amount in minor units (cents, etc.). Must be >= 1.
    pub amount: i64,
    /// The payment method: a closed set.
    pub method: String,
    /// The signer. Mandatory, and never waived.
    pub by: String,
    /// Which till the cash went into (if method is cash). Optional.
    pub till_id: Option<String>,
    /// Advisory: which orders this payment covers, for the UI. Not checked.
    pub covers: Option<Vec<String>>,
    pub now_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayOut {
    /// The order as it now is.
    pub merged: String,
    pub seq: u64,
    pub generation: i64,
}

/// A round in one of these was never owed, or its money is already going back:
/// taking a payment on it would record money the venue must then return.
/// Every other status takes one — a dine-in bill is settled after the food.
fn refuses_payment(status: &str) -> bool {
    matches!(status, "CANCELLED" | "REJECTED" | "REFUNDING" | "COMPENSATED_REFUND")
}

/// The closed set of payment methods. A method name entered by the waiter or
/// printed by the POS, so it is a word, not free text.
pub fn validate_method(method: &str) -> bool {
    matches!(
        method.trim(),
        "cash" | "card" | "cheque" | "transfer" | "gift_card" | "other"
    )
}

/// Fold all payments received so far (including this one). Returns their sum.
fn fold_payments(order: &Value, new_amount: i64) -> Result<i64, Refused> {
    let payments = order
        .get("payments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut total = new_amount;
    for p in &payments {
        let amt = p.get("amount").and_then(Value::as_i64).unwrap_or(0);
        total = total.checked_add(amt).ok_or_else(|| Refused::Invalid("payment sum overflows".into()))?;
    }
    Ok(total)
}

/// THE WHOLE PAYMENT, over the order image already in memory.
pub fn decide(
    hub: &mut dowiz_hub::Hub,
    current: Option<&OrderView>,
    input: &PayIn,
) -> Result<(Value, String, u64), Refused> {
    let Some(current) = current else { return Err(Refused::NotFound) };

    // Rule a: no signer
    if input.by.trim().is_empty() {
        return Err(Refused::Invalid("a payment names who took it".into()));
    }

    // Rule b: amount < 1
    if input.amount < 1 {
        return Err(Refused::Invalid("a payment is at least 1 minor unit".into()));
    }

    // Parse the current order JSON
    let mut order: Value = serde_json::from_str(&current.order_json)
        .map_err(|e| Refused::Append(format!("order json unreadable: {e}")))?;

    // Rule d: another venue's order
    if order.get("location_id").and_then(Value::as_str) != Some(&input.location_id) {
        return Err(Refused::NotFound);
    }

    let status = order.get("status").and_then(Value::as_str).unwrap_or("");
    if refuses_payment(status) {
        return Err(Refused::Conflict(format!("this round is {status}; it takes no payment")));
    }

    // Get the order's total
    let total = order.get("total").and_then(Value::as_i64).unwrap_or(0);

    // Rule c: Σ of all payments > total
    let paid_so_far = fold_payments(&order, input.amount)?;
    if paid_so_far > total {
        return Err(Refused::Conflict(format!(
            "payment sum {paid_so_far} exceeds the total {total}"
        )));
    }

    // Validate the method
    if !validate_method(&input.method) {
        return Err(Refused::Invalid(format!("method {}: not recognized", input.method)));
    }

    // Rule f (law 3): a payment never changes subtotal, discount or total —
    // only `payments` and `payment_status` are written below, and the test
    // `payment_never_changes_the_round_money_fields` holds it.
    // Append the payment to the payments array
    let mut payments = order
        .get("payments")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    payments.push(json!({
        "by": input.by,
        "amount": input.amount,
        "method": input.method,
        "at": input.now_ms,
        "till_id": input.till_id,
    }));
    order["payments"] = Value::Array(payments);

    // Rule e: when Σ payments == total, mark as paid
    if paid_so_far == total {
        order["payment_status"] = json!("paid");
    }

    // Create the delta: only the payments array (and possibly payment_status) changed
    let body = crate::fold::delta(
        &serde_json::from_str::<Value>(&current.order_json)
            .unwrap_or(json!({})),
        &order,
    )
    .to_string();

    let seq = next_seq(current.seq, input.now_ms);
    hub.append(dowiz_hub::EventKind::Paid, &input.order_id, &body, seq, [0u8; 32])
        .map_err(|e| Refused::Append(format!("hub append failed: {e:?}")))?;

    Ok((order, body, seq))
}

#[cfg(test)]
mod tests;
